//! The io-completion callback visa invokes, and the per-job state machine it drives.

use crate::{
    enums::{
        attribute::{self, SpecAttr},
        event,
        status::{CompletionCode, ErrorCode},
    },
    session::FromRawSs,
    Instrument, JobID, Result,
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::task::{Context, Poll, Waker};
use visa_sys as vs;

/// State of one asynchronous operation, keyed by its visa job id.
///
/// Every transition goes through `DashMap::entry`, so the callback thread and the
/// polling thread share one critical section per job. That is what makes "take the
/// result, otherwise park a waker" atomic; with the result and the waker in separate
/// maps there was a window where a completion could be recorded after the waiter had
/// already looked for it, stranding the future forever.
///
/// The pack also owns the transfer's buffer while it is in flight. Visa keeps the raw
/// pointer until it posts the completion event, so the buffer must not be freed before
/// then -- not even when the future is dropped and the job terminated.
/// What a finished transfer reported.
///
/// `completion` is what lets a multi-chunk read know whether the device is done:
/// [`SuccessMaxCnt`](CompletionCode::SuccessMaxCnt) means the buffer filled and more may
/// follow, anything else means the device ended the response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Completed {
    pub(crate) count: usize,
    pub(crate) completion: CompletionCode,
}

pub(super) enum JobState {
    /// In flight, with a future waiting on it.
    Waiting { waker: Waker, buf: Vec<u8> },
    /// Finished. `buf` is `None` only when the completion beat `register` here, in which
    /// case `register` attaches the buffer a moment later.
    Done {
        result: Result<Completed>,
        buf: Option<Vec<u8>>,
    },
    /// Terminated. The buffer is held here, unreachable but alive, until the completion
    /// arrives and proves visa is done writing into it.
    Cancelled { buf: Option<Vec<u8>> },
}

pub(super) struct JobMap {
    jobs: DashMap<JobID, JobState>,
}

impl JobMap {
    pub(super) fn new() -> Self {
        Self {
            jobs: DashMap::new(),
        }
    }

    /// Registers a freshly started job and takes ownership of its buffer.
    ///
    /// The completion may already have landed, because visa can run the callback before
    /// `viReadAsync`/`viWriteAsync` has even returned the job id. In that case the
    /// result is already parked here and only the buffer is missing.
    pub(super) fn register(&self, job_id: JobID, waker: &Waker, buf: Vec<u8>) {
        match self.jobs.entry(job_id) {
            Entry::Vacant(v) => {
                v.insert(JobState::Waiting {
                    waker: waker.clone(),
                    buf,
                });
            }
            Entry::Occupied(mut o) => {
                let awaiting_buf = matches!(o.get(), JobState::Done { buf: None, .. });
                if awaiting_buf {
                    if let JobState::Done { buf: slot, .. } = o.get_mut() {
                        *slot = Some(buf);
                    }
                } else {
                    // A `Cancelled` marker whose job id visa has recycled: a new job.
                    o.insert(JobState::Waiting {
                        waker: waker.clone(),
                        buf,
                    });
                }
            }
        }
    }

    /// Takes the result and buffer of `job_id` if it has finished, otherwise parks the
    /// waker from `cx`.
    ///
    /// The waker is stored under the same lock the callback takes, so a completion
    /// racing this call either sees the new waker or has already stored its result where
    /// this call will find it. There is no window for a lost wakeup.
    pub(super) fn poll_job(
        &self,
        job_id: JobID,
        cx: &Context<'_>,
    ) -> Poll<(Result<Completed>, Option<Vec<u8>>)> {
        match self.jobs.entry(job_id) {
            Entry::Occupied(mut o) => {
                if matches!(o.get(), JobState::Done { .. }) {
                    match o.remove() {
                        JobState::Done { result, buf } => Poll::Ready((result, buf)),
                        _ => unreachable!("checked immediately above"),
                    }
                } else {
                    if let JobState::Waiting { waker, .. } = o.get_mut() {
                        *waker = cx.waker().clone();
                    }
                    Poll::Pending
                }
            }
            // Never registered, or its result was already taken by an earlier poll.
            Entry::Vacant(_) => Poll::Ready((Err(ErrorCode::ErrorInvJobId.into()), None)),
        }
    }

    /// Records the completion of `job_id`. Called from the visa callback thread.
    fn complete(&self, job_id: JobID, result: Result<Completed>) {
        let waker = match self.jobs.entry(job_id) {
            // Beat `register` here; park the result for it.
            Entry::Vacant(v) => {
                v.insert(JobState::Done { result, buf: None });
                None
            }
            Entry::Occupied(mut o) => {
                // Placeholder: every arm below either overwrites this or removes the entry.
                let prev = std::mem::replace(o.get_mut(), JobState::Cancelled { buf: None });
                match prev {
                    // The job was terminated and this is the completion we were holding
                    // the buffer for. Dropping the entry frees it -- visa is done now.
                    JobState::Cancelled { .. } => {
                        o.remove();
                        None
                    }
                    // Visa posts exactly one completion per job, so a second means a
                    // recycled id or a driver bug. Keep the first result.
                    JobState::Done { result, buf } => {
                        log::warn!("duplicate io completion for job {}, ignored", job_id.0);
                        *o.get_mut() = JobState::Done { result, buf };
                        None
                    }
                    JobState::Waiting { waker, buf } => {
                        *o.get_mut() = JobState::Done {
                            result,
                            buf: Some(buf),
                        };
                        Some(waker)
                    }
                }
            }
        };
        // Outside the entry guard: waking can poll the future on this very thread, which
        // would deadlock against the shard lock held above.
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Marks `job_id` terminated, keeping its buffer alive until the completion arrives.
    ///
    /// Self-cleaning: visa always posts a completion for a terminated job, and that is
    /// what removes the entry and releases the buffer.
    /// Leaks the buffer of every job visa has not reported finished, returning how many.
    ///
    /// A `Waiting` or `Cancelled` entry means visa was handed the buffer's pointer and
    /// has not yet posted the completion that proves it stopped writing. By the time
    /// this runs the handler is uninstalled and `viClose` has been called, but NI
    /// documents no guarantee that closing a session terminates outstanding transfers --
    /// so freeing these would be a free we cannot prove is safe. A leak always is.
    fn leak_unfinished_buffers(&self) -> usize {
        let mut leaked = 0;
        for mut entry in self.jobs.iter_mut() {
            let state = entry.value_mut();
            let buf = match state {
                JobState::Waiting { buf, .. } => Some(std::mem::take(buf)),
                JobState::Cancelled { buf } => buf.take(),
                // The completion arrived, so visa is provably done with this one.
                JobState::Done { .. } => None,
            };
            if let Some(buf) = buf {
                std::mem::forget(buf);
                // Mark the entry as holding nothing so a second pass cannot count it
                // again -- `mem::take` leaves an empty `Vec<u8>`, not an absence.
                *state = JobState::Cancelled { buf: None };
                leaked += 1;
            }
        }
        leaked
    }

    pub(super) fn cancel(&self, job_id: JobID) {
        match self.jobs.entry(job_id) {
            Entry::Vacant(v) => {
                v.insert(JobState::Cancelled { buf: None });
            }
            Entry::Occupied(mut o) => {
                let prev = std::mem::replace(o.get_mut(), JobState::Cancelled { buf: None });
                match prev {
                    // Already finished, so nothing is still coming and nothing is still
                    // reading the buffer; drop both now.
                    JobState::Done { .. } => {
                        o.remove();
                    }
                    JobState::Waiting { buf, .. } => {
                        *o.get_mut() = JobState::Cancelled { buf: Some(buf) };
                    }
                    JobState::Cancelled { buf } => {
                        *o.get_mut() = JobState::Cancelled { buf };
                    }
                }
            }
        }
    }
    fn call(&self, _instr: &Instrument, event: &event::Event) -> vs::ViStatus {
        let status = match attribute::AttrStatus::get_from(event) {
            Ok(status) => status,
            Err(e) => {
                log::error!("error checking status in async io callback: {}", e);
                return vs::VI_SUCCESS as _;
            }
        };
        let job_id = match attribute::AttrJobId::get_from(event) {
            Ok(id) => JobID(id.into_inner()),
            Err(e) => {
                log::error!("error checking job id in async io callback: {}", e);
                return vs::VI_SUCCESS as _;
            }
        };
        let completion = CompletionCode::try_from(status).map_err(crate::Error::from);
        if let Ok(c) = completion {
            match c {
                CompletionCode::WarnQueueOverflow => log::warn!("queue overflow in async io"),
                CompletionCode::Success
                | CompletionCode::SuccessSync
                | CompletionCode::SuccessMaxCnt
                | CompletionCode::SuccessTermChar
                | CompletionCode::SuccessQueueEmpty
                | CompletionCode::SuccessQueueNempty => {}
                other => log::warn!("unexpected completion code for async io: {}", other),
            }
        }
        let result = completion.and_then(|completion| {
            attribute::AttrRetCount::get_from(event).map(|c| Completed {
                count: c.into_inner() as _,
                completion,
            })
        });
        if let Err(ref e) = result {
            log::debug!("async io completed with error: job={}, {}", job_id.0, e);
        }
        self.complete(job_id, result);
        vs::VI_SUCCESS_NCHAIN as _
        //Normally, an application should always return VI_SUCCESS from all callback handlers. If a specific handler does not want other handlers to be invoked for the given event for the given session, it should return VI_SUCCESS_NCHAIN.
    }

    pub(super) unsafe extern "system" fn call_in_c(
        instr: vs::ViSession,
        event_type: vs::ViEventType,
        event: vs::ViEvent,
        user_data: *mut std::ffi::c_void,
    ) -> vs::ViStatus {
        // Unwinding out of an `extern "system"` fn aborts the process, and visa gives us
        // no way to report a panic, so contain it here and tell visa the event was handled.
        let ret = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // A shared reference, never `&mut`: visa invokes this on its own thread while
            // the application thread is holding `&JobMap` (see `register`), so a unique
            // reference here would break the aliasing rules. Every field carries its own
            // interior mutability, so `&Self` is all `call` needs.
            let pack: &Self = &*(user_data as *const Self);
            // `ManuallyDrop`: neither of these owns what it wraps, and a panic must not
            // let their `Drop` run `viClose` on the caller's session or on an event
            // context visa frees itself.
            let instr = std::mem::ManuallyDrop::new(Instrument::from_raw_ss(instr));
            let event = std::mem::ManuallyDrop::new(event::Event::new(event, event_type));
            pack.call(&instr, &event)
        }));
        match ret {
            Ok(ret) => ret,
            Err(_) => {
                log::error!("panic in visa io completion handler, ignored");
                vs::VI_SUCCESS as _
            }
        }
    }
}

impl Drop for JobMap {
    fn drop(&mut self) {
        let leaked = self.leak_unfinished_buffers();
        if leaked > 0 {
            log::warn!(
                "leaked {} buffer(s): visa never reported these transfers finished",
                leaked
            );
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Barrier,
    };
    use std::task::Wake;

    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn done(count: usize) -> Result<Completed> {
        Ok(Completed {
            count,
            completion: CompletionCode::Success,
        })
    }

    fn waker() -> (Arc<CountingWaker>, Waker) {
        let w = Arc::new(CountingWaker(AtomicUsize::new(0)));
        (w.clone(), Waker::from(w))
    }

    fn ready(
        p: Poll<(Result<Completed>, Option<Vec<u8>>)>,
    ) -> (Result<Completed>, Option<Vec<u8>>) {
        match p {
            Poll::Ready(v) => v,
            Poll::Pending => panic!("expected the job to be ready"),
        }
    }
    /// Visa can invoke the callback before `viReadAsync` has even returned the job id,
    /// so a completion may be recorded before anything is waiting for it.
    #[test]
    fn completion_before_register_is_not_lost() {
        let pack = JobMap::new();
        let id = JobID(7);
        let (count, w) = waker();
        pack.complete(id, done(12));
        pack.register(id, &w, vec![0u8; 16]);
        let cx = Context::from_waker(&w);
        let (result, buf) = ready(pack.poll_job(id, &cx));
        assert_eq!(result.unwrap().count, 12);
        assert_eq!(buf.expect("buffer handed back").len(), 16);
        assert_eq!(
            count.0.load(Ordering::SeqCst),
            0,
            "nothing was waiting to wake"
        );
    }

    #[test]
    fn register_then_complete_wakes_and_delivers() {
        let pack = JobMap::new();
        let id = JobID(1);
        let (count, w) = waker();
        let cx = Context::from_waker(&w);
        pack.register(id, &w, vec![0u8; 8]);
        assert!(pack.poll_job(id, &cx).is_pending());
        pack.complete(id, done(5));
        assert_eq!(count.0.load(Ordering::SeqCst), 1);
        let (result, buf) = ready(pack.poll_job(id, &cx));
        assert_eq!(result.unwrap().count, 5);
        assert_eq!(buf.expect("buffer handed back").len(), 8);
    }

    /// The reason the pack owns the buffer at all: after `viTerminate` visa may still be
    /// writing into it, so it must outlive cancellation and only go once the completion
    /// proves visa is finished with it.
    #[test]
    fn cancelled_job_holds_its_buffer_until_the_completion_arrives() {
        let pack = JobMap::new();
        let id = JobID(2);
        let (count, w) = waker();
        pack.register(id, &w, vec![0u8; 64]);
        pack.cancel(id);
        {
            let entry = pack.jobs.get(&id).expect("entry retained after cancel");
            assert!(
                matches!(*entry, JobState::Cancelled { buf: Some(_) }),
                "buffer must still be alive while visa may write to it"
            );
        }
        pack.complete(id, done(9));
        assert_eq!(count.0.load(Ordering::SeqCst), 0, "no future to wake");
        assert!(
            pack.jobs.is_empty(),
            "buffer released once visa reported done"
        );
    }

    /// A fixed-size cancelled ring used to evict ids and then mis-deliver a stale result
    /// to whichever job visa handed that id to next.
    #[test]
    fn recycled_job_id_after_cancel_is_a_fresh_job() {
        let pack = JobMap::new();
        let id = JobID(3);
        pack.cancel(id);
        let (_c, w) = waker();
        let cx = Context::from_waker(&w);
        pack.register(id, &w, vec![0u8; 4]);
        assert!(pack.poll_job(id, &cx).is_pending());
        pack.complete(id, done(4));
        assert_eq!(ready(pack.poll_job(id, &cx)).0.unwrap().count, 4);
    }

    #[test]
    fn polling_an_unknown_job_errors_rather_than_hanging() {
        let pack = JobMap::new();
        let (_c, w) = waker();
        let cx = Context::from_waker(&w);
        assert!(matches!(
            ready(pack.poll_job(JobID(99), &cx)).0,
            Err(crate::Error(ErrorCode::ErrorInvJobId))
        ));
    }

    #[test]
    fn duplicate_completion_keeps_the_first_result() {
        let pack = JobMap::new();
        let id = JobID(4);
        pack.complete(id, done(1));
        pack.complete(id, done(999));
        let (_c, w) = waker();
        let cx = Context::from_waker(&w);
        assert_eq!(ready(pack.poll_job(id, &cx)).0.unwrap().count, 1);
    }

    /// Tearing the session down with transfers visa never reported finished must leak
    /// their buffers, not free them: `viClose` is not documented to stop the driver
    /// writing, and an unprovable free is worse than a bounded leak.
    #[test]
    fn unfinished_buffers_are_leaked_not_freed() {
        let pack = JobMap::new();
        let (_c, w) = waker();

        // one still in flight, one terminated but not yet completed, one finished
        pack.register(JobID(1), &w, vec![0u8; 8]);
        pack.register(JobID(2), &w, vec![0u8; 8]);
        pack.cancel(JobID(2));
        pack.register(JobID(3), &w, vec![0u8; 8]);
        pack.complete(JobID(3), done(8));

        assert_eq!(
            pack.leak_unfinished_buffers(),
            2,
            "jobs 1 and 2 are unproven"
        );
        // the completed one keeps its buffer, and it is still handed back
        let cx = Context::from_waker(&w);
        assert!(ready(pack.poll_job(JobID(3), &cx)).1.is_some());
        // running twice must not double-leak
        assert_eq!(pack.leak_unfinished_buffers(), 0);
    }

    /// Exercises the register/complete interleaving that could once strand a result
    /// between the `jobs` and `pending` maps and hang the future forever.
    #[test]
    fn concurrent_register_and_complete_never_strands() {
        let pack = Arc::new(JobMap::new());
        for i in 0..1000u32 {
            let id = JobID(i as _);
            let (_c, w) = waker();
            let barrier = Arc::new(Barrier::new(2));
            let (p2, b2) = (pack.clone(), barrier.clone());
            let h = std::thread::spawn(move || {
                b2.wait();
                p2.complete(id, done(i as usize));
            });
            barrier.wait();
            pack.register(id, &w, vec![0u8; 2]);
            h.join().unwrap();
            let cx = Context::from_waker(&w);
            let (result, buf) = ready(pack.poll_job(id, &cx));
            assert_eq!(
                result.unwrap().count,
                i as usize,
                "completion for job {i} was stranded"
            );
            assert!(buf.is_some(), "buffer for job {i} was lost");
        }
        assert!(pack.jobs.is_empty());
    }
}
