use super::*;
/// Session to a specified resource
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Instrument(pub(crate) OwnedSs);

impl std::io::Write for Instrument {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        <&Instrument>::write(&mut &*self, buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        <&Instrument>::flush(&mut &*self)
    }
}

impl std::io::Read for Instrument {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        <&Instrument>::read(&mut &*self, buf)
    }
}

impl std::io::Write for &Instrument {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut ret_cnt: vs::ViUInt32 = 0;
        wrap_raw_error_in_unsafe!(vs::viWrite(
            self.as_raw_ss(),
            buf.as_ptr(),
            buf.len() as _,
            &mut ret_cnt as _
        ))
        .map_err(vs_to_io_err)?;

        Ok(ret_cnt as _)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.visa_flush(flags::FlushMode::IO_OUT_BUF)
            .map_err(vs_to_io_err)
        // Flush the low-level I/O buffer used by viWrite
    }
}

impl std::io::Read for &Instrument {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut ret_cnt: vs::ViUInt32 = 0;
        wrap_raw_error_in_unsafe!(vs::viRead(
            self.as_raw_ss(),
            buf.as_mut_ptr(),
            buf.len() as _,
            &mut ret_cnt as _
        ))
        .map_err(vs_to_io_err)?;
        Ok(ret_cnt as _)
    }
}

impl Instrument {
    ///Manually flushes the specified buffers associated with formatted I/O operations and/or serial communication.
    pub fn visa_flush(&self, mode: flags::FlushMode) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viFlush(self.as_raw_ss(), mode.bits()))?;
        Ok(())
    }
    /// Returns a user-readable description of the status code passed to the operation.
    pub fn status_desc(&self, error: Error) -> Result<VisaString> {
        let mut desc: VisaBuf = new_visa_buf();
        wrap_raw_error_in_unsafe!(vs::viStatusDesc(
            self.as_raw_ss(),
            error.into(),
            desc.as_mut_ptr() as _
        ))?;
        Ok(desc.try_into().unwrap())
    }
    /// Establishes an access mode to the specified resources.
    ///
    /// This operation is used to obtain a lock on the specified resource. The caller can specify the type of lock requested—exclusive or shared lock—and the length of time the operation will suspend while waiting to acquire the lock before timing out. This operation can also be used for sharing and nesting locks.
    ///
    /// The session that gained a shared lock can pass the accessKey to other sessions for the purpose of sharing the lock. The session wanting to join the group of sessions sharing the lock can use the key as an input value to the requestedKey parameter. VISA will add the session to the list of sessions sharing the lock, as long as the requestedKey value matches the accessKey value for the particular resource. The session obtaining a shared lock in this manner will then have the same access privileges as the original session that obtained the lock.
    ///
    ///It is also possible to obtain nested locks through this operation. To acquire nested locks, invoke the viLock() operation with the same lock type as the previous invocation of this operation. For each session, viLock() and viUnlock() share a lock count, which is initialized to 0. Each invocation of viLock() for the same session (and for the same lockType) increases the lock count. In the case of a shared lock, it returns with the same accessKey every time. When a session locks the resource a multiple number of times, it is necessary to invoke the viUnlock() operation an equal number of times in order to unlock the resource. That is, the lock count increments for each invocation of viLock(), and decrements for each invocation of viUnlock(). A resource is actually unlocked only when the lock count is 0.
    ///
    ///The VISA locking mechanism enforces arbitration of accesses to resources on an individual basis. If a session locks a resource, operations invoked by other sessions to the same resource are serviced or returned with a locking error, depending on the operation and the type of lock used. If a session has an exclusive lock, other sessions cannot modify global attributes or invoke operations, but can still get attributes and set local attributes. If the session has a shared lock, other sessions that have shared locks can also modify global attributes and invoke operations. Regardless of which type of lock a session has, if the session is closed without first being unlocked, VISA automatically performs a viUnlock() on that session.
    ///
    ///The locking mechanism works for all processes and resources existing on the same computer. When using remote resources, however, the networking protocol may not provide the ability to pass lock requests to the remote device or resource. In this case, locks will behave as expected from multiple sessions on the same computer, but not necessarily on the remote device. For example, when using the VXI-11 protocol, exclusive lock requests can be sent to a device, but shared locks can only be handled locally.
    ///
    /// see also [`Self::lock_exclusive`], [`Self::lock_shared`] and [`Self::lock_shared_with_key`]
    ///
    pub fn lock(
        &self,
        mode: flags::AccessMode,
        timeout: Duration,
        key: Option<AccessKey>,
    ) -> Result<Option<AccessKey>> {
        if (mode & flags::AccessMode::SHARED_LOCK).is_empty() {
            wrap_raw_error_in_unsafe!(vs::viLock(
                self.as_raw_ss(),
                mode.bits(),
                timeout.as_millis() as _,
                vs::VI_NULL as _,
                vs::VI_NULL as _
            ))?;
            Ok(None)
        } else {
            let mut ak = new_visa_buf();
            wrap_raw_error_in_unsafe!(vs::viLock(
                self.as_raw_ss(),
                mode.bits(),
                timeout.as_millis() as _,
                key.map(|x| x.as_vi_const_string())
                    .unwrap_or(vs::VI_NULL as _),
                ak.as_mut_ptr() as _
            ))?;
            Ok(Some(ak.try_into().unwrap()))
        }
    }

    pub fn lock_exclusive(&self, timeout: Duration) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viLock(
            self.as_raw_ss(),
            flags::AccessMode::EXCLUSIVE_LOCK.bits(),
            timeout.as_millis() as _,
            vs::VI_NULL as _,
            vs::VI_NULL as _
        ))?;
        Ok(())
    }

    pub fn lock_shared(&self, timeout: Duration) -> Result<AccessKey> {
        let mut ak = new_visa_buf();
        wrap_raw_error_in_unsafe!(vs::viLock(
            self.as_raw_ss(),
            flags::AccessMode::EXCLUSIVE_LOCK.bits(),
            timeout.as_millis() as _,
            vs::VI_NULL as _,
            ak.as_mut_ptr() as _
        ))?;
        Ok(ak.try_into().unwrap())
    }

    pub fn lock_shared_with_key(&self, timeout: Duration, key: AccessKey) -> Result<AccessKey> {
        let mut ak = new_visa_buf();
        wrap_raw_error_in_unsafe!(vs::viLock(
            self.as_raw_ss(),
            flags::AccessMode::EXCLUSIVE_LOCK.bits(),
            timeout.as_millis() as _,
            key.as_vi_const_string() as _,
            ak.as_mut_ptr() as _
        ))?;
        Ok(ak.try_into().unwrap())
    }

    ///Relinquishes a lock for the specified resource.
    pub fn unlock(&self) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viUnlock(self.as_raw_ss()))?;
        Ok(())
    }

    ///Enables notification of a specified event.
    ///
    ///The specified session can be enabled to queue events by specifying VI_QUEUE. Applications can enable the session to invoke a callback function to execute the handler by specifying VI_HNDLR. The applications are required to install at least one handler to be enabled for this mode. Specifying VI_SUSPEND_HNDLR enables the session to receive callbacks, but the invocation of the handler is deferred to a later time. Successive calls to this operation replace the old callback mechanism with the new callback mechanism.
    ///
    ///Specifying VI_ALL_ENABLED_EVENTS for the eventType parameter refers to all events which have previously been enabled on this session, making it easier to switch between the two callback mechanisms for multiple events.
    ///
    /// NI-VISA does not support enabling both the queue and the handler for the same event type on the same session. If you need to use both mechanisms for the same event type, you should open multiple sessions to the resource.
    pub fn enable_event(
        &self,
        event_kind: event::EventKind,
        mechanism: event::Mechanism,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viEnableEvent(
            self.as_raw_ss(),
            event_kind as _,
            mechanism as _,
            event::EventFilter::Null as _
        ))?;
        Ok(())
    }

    /// Disables notification of the specified event type(s) via the specified mechanism(s).
    ///
    /// The viDisableEvent() operation disables servicing of an event identified by the eventType parameter for the mechanisms specified in the mechanism parameter. This operation prevents new event occurrences from being added to the queue(s). However, event occurrences already existing in the queue(s) are not flushed. Use viDiscardEvents() if you want to discard events remaining in the queue(s).
    ///
    /// Specifying VI_ALL_ENABLED_EVENTS for the eventType parameter allows a session to stop receiving all events. The session can stop receiving queued events by specifying VI_QUEUE. Applications can stop receiving callback events by specifying either VI_HNDLR or VI_SUSPEND_HNDLR. Specifying VI_ALL_MECH disables both the queuing and callback mechanisms.
    ///
    pub fn disable_event(
        &self,
        event_kind: event::EventKind,
        mechanism: event::Mechanism,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viDisableEvent(
            self.as_raw_ss(),
            event_kind as _,
            mechanism as _,
        ))?;
        Ok(())
    }
    /// Discards event occurrences for specified event types and mechanisms in a session.
    ///
    /// The viDiscardEvents() operation discards all pending occurrences of the specified event types and mechanisms from the specified session. Specifying VI_ALL_ENABLED_EVENTS for the eventType parameter discards events of every type that is enabled for the given session.
    ///
    /// The information about all the event occurrences which have not yet been handled is discarded. This operation is useful to remove event occurrences that an application no longer needs. The discarded event occurrences are not available to a session at a later time.
    ///
    /// This operation does not apply to event contexts that have already been delivered to the application.
    pub fn discard_events(
        &self,
        event: event::EventKind,
        mechanism: event::Mechanism,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viDiscardEvents(
            self.as_raw_ss(),
            event as _,
            mechanism as _,
        ))?;
        Ok(())
    }
    /// Waits for an occurrence of the specified event for a given session.
    ///
    /// The viWaitOnEvent() operation suspends the execution of a thread of an application and waits for an event of the type specified by inEventType for a time period specified by timeout. You can wait only for events that have been enabled with the viEnableEvent() operation. Refer to individual event descriptions for context definitions. If the specified inEventType is VI_ALL_ENABLED_EVENTS, the operation waits for any event that is enabled for the given session. If the specified timeout value is VI_TMO_INFINITE, the operation is suspended indefinitely. If the specified timeout value is VI_TMO_IMMEDIATE, the operation is not suspended; therefore, this value can be used to dequeue events from an event queue.
    ///
    /// When the outContext handle returned from a successful invocation of viWaitOnEvent() is no longer needed, it should be passed to viClose().
    ///
    /// If a session's event queue becomes full and a new event arrives, the new event is discarded. The default event queue size (per session) is 50, which is sufficiently large for most  applications. If an application expects more than 50 events to arrive without having been handled, it can modify the value of the attribute VI_ATTR_MAX_QUEUE_LENGTH to the required size.
    pub fn wait_on_event(
        &self,
        event_kind: event::EventKind,
        timeout: Duration,
    ) -> Result<event::Event> {
        let mut handler: vs::ViEvent = 0;
        let mut out_kind: vs::ViEventType = 0;
        wrap_raw_error_in_unsafe!(vs::viWaitOnEvent(
            self.as_raw_ss(),
            event_kind as _,
            timeout.as_millis() as _,
            &mut out_kind as _,
            &mut handler as _
        ))?;
        let kind = event::EventKind::try_from(out_kind).expect("should be valid event type");
        Ok(event::Event { handler, kind })
    }

    ///
    /// Installs handlers for event callbacks.
    ///
    /// The viInstallHandler() operation allows applications to install handlers on sessions. The handler specified in the handler parameter is installed along with any previously installed handlers for the specified event.
    ///
    /// VISA allows applications to install multiple handlers for an eventType on the same session. You can install multiple handlers through multiple invocations of the viInstallHandler() operation, where each invocation adds to the previous list of handlers. If more than one handler is installed for an eventType, each of the handlers is invoked on every occurrence of the specified event(s). VISA specifies that the handlers are invoked in Last In First Out (LIFO) order.
    ///
    /// *Note*: for some reason pass a closure with type `|instr, event|{...}` may get error.
    /// Instead, use `|instr: & Instrument, event: & Event|{...}`.
    ///

    pub fn install_handler<F: handler::Callback>(
        &self,
        event_kind: event::EventKind,
        callback: F,
    ) -> Result<handler::Handler<'_, F>> {
        handler::Handler::new(self.as_ss(), event_kind, callback)
    }

    /// Reads a status byte of the service request.
    ///
    /// The IEEE 488.2 standard defines several bit assignments in the status byte. For example, if bit 6 of the status is set, the device is requesting service. In addition to setting bit 6 when requesting service, 488.2 devices also use two other bits to specify their status. Bit 4, the Message Available bit (MAV), is set when the device is ready to send previously queried data. Bit 5, the Event Status bit (ESB), is set if one or more of the enabled 488.2 events occurs. These events include power-on, user request, command error, execution error, device dependent error, query error, request control, and operation complete. The device can assert SRQ when ESB or MAV are set, or when a manufacturer-defined condition occurs. Manufacturers of 488.2 devices use the remaining lower-order bits to communicate the reason for the service request or to summarize the device state.
    ///
    pub fn read_stb(&self) -> Result<u16> {
        let mut stb = 0;
        wrap_raw_error_in_unsafe!(vs::viReadSTB(self.as_raw_ss(), &mut stb as *mut _))?;
        Ok(stb)
    }

    /// The viClear() operation clears the device input and output buffers.
    pub fn clear(&self) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viClear(self.as_raw_ss()))?;
        Ok(())
    }

    /// Asserts the specified interrupt or signal.
    ///
    /// This operation can be used to assert a device interrupt condition. In VXI, for example, this can be done with either a VXI signal or a VXI interrupt. On certain bus types, the statusID parameter may be ignored.
    /// statusID: This is the status value to be presented during an interrupt acknowledge cycle.
    ///
    /// # Warning:
    /// I'm not sure if should use [CompletionCode](crate::enums::status::CompletionCode) as status_id as I'm not familiar with this function, let me know if you can help
    pub fn assert_intr_signal(
        &self,
        how: enums::assert::AssertIntrHow,
        status_id: vs::ViUInt32,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viAssertIntrSignal(
            self.as_raw_ss(),
            how as _,
            status_id as _
        ))?;
        Ok(())
    }
    /// Asserts software or hardware trigger.
    ///
    /// The viAssertTrigger() operation sources a software or hardware trigger dependent on the interface type.
    ///
    /// # Software Triggers for 488.2 Instruments (GPIB, VXI, TCPIP, and USB)
    /// This operation sends an IEEE-488.2 software trigger to the addressed device. For software triggers, VI_TRIG_PROT_DEFAULT is the only valid protocol. The bus-specific details are:
    ///
    /// + For a GPIB device, VISA addresses the device to listen and then sends the GPIB GET command.
    /// + For a VXI device, VISA sends the Word Serial Trigger command.
    /// + For a USB device, VISA sends the TRIGGER message ID on the Bulk-OUT pipe.
    ///
    /// # Software Triggers for Non-488.2 Instruments (Serial INSTR, TCPIP SOCKET, and USB RAW)
    /// If VI_ATTR_IO_PROT is VI_PROT_4882_STRS, this operations sends "*TRG\n" to the device; otherwise, this operation is not valid. For software triggers, VI_TRIG_PROT_DEFAULT is the only valid protocol.
    ///
    /// # Hardware Triggering for VXI
    /// For hardware triggers to VXI instruments, VI_ATTR_TRIG_ID must first be set to the desired trigger line to use; this operation performs the specified trigger operation on the previously selected trigger line. For VXI hardware triggers, VI_TRIG_PROT_DEFAULT is equivalent to VI_TRIG_PROT_SYNC.
    ///
    /// # Trigger Reservation for PXI
    /// For PXI instruments, this operation reserves or releases (unreserves) a trigger line for use in external triggering. For PXI triggers, VI_TRIG_PROT_RESERVE and VI_TRIG_PROT_UNRESERVE are the only valid protocols.
    pub fn assert_trigger(&self, protocol: enums::assert::AssertTrigPro) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viAssertTrigger(self.as_raw_ss(), protocol as _))?;
        Ok(())
    }

    /// Asserts or deasserts the specified utility bus signal.
    ///
    /// This operation can be used to assert either the SYSFAIL or SYSRESET utility bus interrupts on the VXIbus backplane. This operation is valid only on BACKPLANE (mainframe) and VXI SERVANT (servant) sessions.
    ///
    /// Asserting SYSRESET (also known as HARD RESET in the VXI specification) should be used only when it is necessary to promptly terminate operation of all devices in a VXIbus system. This is a serious action that always affects the entire VXIbus system.
    pub fn assert_util_signal(&self, line: enums::assert::AssertBusSignal) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viAssertUtilSignal(self.as_raw_ss(), line as _))?;
        Ok(())
    }

    /// Reads data from device or interface through the use of a formatted I/O read buffer.
    ///
    /// The viBufRead() operation is similar to viRead() and does not perform any kind of data formatting. It differs from viRead() in that the data is read from the formatted I/O read buffer—the same buffer used by viScanf() and related operations—rather than directly from the device. You can intermix this operation with viScanf(), but you should not mix it with viRead().
    ///
    /// * Note: If `buf` is empty, the `retCount` in [viBufRead](vs::viBufRead) is set to [VI_NULL](vs::VI_NULL), the number of bytes transferred is not returned. You may find this useful if you need to know only whether the operation succeeded or failed.
    pub fn buf_read(&self, buf: &mut [u8]) -> Result<usize> {
        let mut ret_cnt: vs::ViUInt32 = 0;
        wrap_raw_error_in_unsafe!(vs::viBufRead(
            self.as_raw_ss(),
            if !buf.is_empty() {
                buf.as_mut_ptr()
            } else {
                vs::VI_NULL as _
            },
            buf.len() as _,
            &mut ret_cnt as _
        ))?;
        Ok(ret_cnt as _)
    }

    /// Writes data to a formatted I/O write buffer synchronously.
    ///
    /// The viBufWrite() operation is similar to viWrite() and does not perform any kind of data formatting. It differs from viWrite() in that the data is written to the formatted I/O write buffer—the same buffer used by viPrintf() and related operations—rather than directly to the device. You can intermix this operation with viPrintf(), but you should not mix it with viWrite().
    ///
    /// If this operation returns VI_ERROR_TMO, the write buffer for the specified session is cleared.
    ///
    /// * Note: If `buf` is empty, the `retCount` in [viBufWrite](vs::viBufWrite) is set to [VI_NULL](vs::VI_NULL), the number of bytes transferred is not returned. You may find this useful if you need to know only whether the operation succeeded or failed.
    pub fn buf_write(&self, buf: &[u8]) -> Result<usize> {
        let mut ret_cnt: vs::ViUInt32 = 0;
        wrap_raw_error_in_unsafe!(vs::viBufWrite(
            self.as_raw_ss(),
            if !buf.is_empty() {
                buf.as_ptr()
            } else {
                vs::VI_NULL as _
            },
            buf.len() as _,
            &mut ret_cnt as _
        ))?;
        Ok(ret_cnt as _)
    }

    /// Sets the size for the formatted I/O and/or low-level I/O communication buffer(s).
    pub fn set_buf(&self, mask: flags::BufMask, size: usize) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viSetBuf(self.as_raw_ss(), mask.bits(), size as _))?;
        Ok(())
    }
}

use crate::async_io;

impl Instrument {
    /// Reads data from device or interface asynchronously.
    ///
    /// The viReadAsync() operation asynchronously transfers data. The data read is to be stored in the buffer represented by buf. This operation normally returns before the transfer terminates.
    ///
    /// Before calling this operation, you should enable the session for receiving I/O completion events. After the transfer has completed, an I/O completion event is posted.
    ///
    /// The operation returns jobId, which you can use with either viTerminate() to abort the operation, or with an I/O completion event to identify which asynchronous read operation completed. VISA will never return VI_NULL for a valid jobID.
    ///
    /// If you have enabled VI_EVENT_IO_COMPLETION for queueing (VI_QUEUE), for each successful call to viReadAsync(), you must call viWaitOnEvent() to retrieve the I/O completion event. This is true even if the I/O is done synchronously (that is, if the operation returns VI_SUCCESS_SYNC).
    /// # Safety
    /// This function is unsafe because the `buf` passed in may be dropped before the transfer terminates

    //todo: return VI_SUCCESS_SYNC, means IO operation has finished, so if there is a waker receiving JobID, would be called before JobID set and can't wake corresponding job
    pub unsafe fn visa_read_async(&self, buf: &mut [u8]) -> Result<JobID> {
        let mut id: vs::ViJobId = 0;
        #[allow(unused_unsafe)]
        wrap_raw_error_in_unsafe!(vs::viReadAsync(
            self.as_raw_ss(),
            buf.as_mut_ptr(),
            buf.len() as _,
            &mut id as _
        ))?;
        Ok(JobID(id))
    }

    /// The viWriteAsync() operation asynchronously transfers data. The data to be written is in the buffer represented by buf. This operation normally returns before the transfer terminates.
    ///
    /// Before calling this operation, you should enable the session for receiving I/O completion events. After the transfer has completed, an I/O completion event is posted.
    ///
    /// The operation returns a job identifier that you can use with either viTerminate() to abort the operation or with an I/O completion event to identify which asynchronous write operation completed. VISA will never return VI_NULL for a valid jobId.
    ///
    /// If you have enabled VI_EVENT_IO_COMPLETION for queueing (VI_QUEUE), for each successful call to viWriteAsync(), you must call viWaitOnEvent() to retrieve the I/O completion event. This is true even if the I/O is done synchronously (that is, if the operation returns VI_SUCCESS_SYNC).
    ///
    /// # Safety
    /// This function is unsafe because the `buf` passed in may be dropped before the transfer terminates

    //todo: return VI_SUCCESS_SYNC, means IO operation has finished, so if there is a waker receiving JobID, would be called before JobID set and can't wake corresponding job

    pub unsafe fn visa_write_async(&self, buf: &[u8]) -> Result<JobID> {
        let mut id: vs::ViJobId = 0;
        #[allow(unused_unsafe)]
        wrap_raw_error_in_unsafe!(vs::viWriteAsync(
            self.as_raw_ss(),
            buf.as_ptr(),
            buf.len() as _,
            &mut id as _
        ))?;
        Ok(JobID(id))
    }

    /// Requests session to terminate normal execution of an operation.
    ///
    /// This operation is used to request a session to terminate normal execution of an operation, as specified by the jobId parameter. The jobId parameter is a unique value generated from each call to an asynchronous operation.
    ///
    /// If a user passes VI_NULL as the jobId value to viTerminate(), VISA will abort any calls in the current process executing on the specified vi. Any call that is terminated this way should return VI_ERROR_ABORT. Due to the nature of multi-threaded systems, for example where operations in other threads may complete normally before the operation viTerminate() has any effect, the specified return value is not guaranteed.
    ///
    pub fn terminate(&self, job_id: JobID) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viTerminate(
            self.as_raw_ss(),
            vs::VI_NULL as _,
            job_id.0
        ))?;
        Ok(())
    }
    /// Safe rust wrapper of [`Self::visa_read_async`]
    ///
    /// *Note*: for now this function returns a future holding reference of `buf` and `Self`,
    /// which means it can't be send to another thread
    pub async fn async_read(&self, buf: &mut [u8]) -> Result<usize> {
        async_io::AsyncRead::new(self, buf).await
    }
    /// Safe rust wrapper of [`Self::visa_write_async`]
    ///
    /// *Note*: for now this function returns a future holding reference of `buf` and `Self`,
    /// which means it can't be send to another thread
    pub async fn async_write(&self, buf: &[u8]) -> Result<usize> {
        async_io::AsyncWrite::new(self, buf).await
    }
}

// GPIB operations
impl Instrument {
    /// Write GPIB command bytes on the bus.
    ///
    /// This operation attempts to write count number of bytes of GPIB commands to the interface bus specified by vi. This operation is valid only on GPIB INTFC (interface) sessions. This operation returns only when the transfer terminates.
    ///
    /// * Note: If `buf` is empty, the `retCount` in [viGpibCommand](vs::viGpibCommand) is set to [VI_NULL](vs::VI_NULL), the number of bytes transferred is not returned. You may find this useful if you need to know only whether the operation succeeded or failed.
    pub fn gpib_command(&self, buf: &[u8]) -> Result<usize> {
        let mut ret_cnt: vs::ViUInt32 = 0;
        wrap_raw_error_in_unsafe!(vs::viGpibCommand(
            self.as_raw_ss(),
            if !buf.is_empty() {
                buf.as_ptr()
            } else {
                vs::VI_NULL as _
            },
            buf.len() as _,
            &mut ret_cnt as _
        ))?;
        Ok(ret_cnt as _)
    }

    /// Specifies the state of the ATN line and the local active controller state.
    ///
    /// This operation asserts or deasserts the GPIB ATN interface line according to the specified mode. The mode can also specify whether the local interface should acquire or release Controller Active status. This operation is valid only on GPIB INTFC (interface) sessions.
    ///
    /// It is generally not necessary to use the viGpibControlATN() operation in most applications. Other operations such as viGpibCommand() and viGpibPassControl() modify the ATN and/or CIC state automatically.
    pub fn gpib_control_atn(&self, mode: enums::gpib::AtnMode) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viGpibControlATN(self.as_raw_ss(), mode as _))?;
        Ok(())
    }

    /// Controls the state of the GPIB Remote Enable (REN) interface line, and optionally the remote/local state of the device.
    ///
    /// The viGpibControlREN() operation asserts or unasserts the GPIB REN interface line according to the specified mode. The mode can also specify whether the device associated with this session should be placed in local state (before deasserting REN) or remote state (after asserting REN). This operation is valid only if the GPIB interface associated with the session specified by vi is currently the system controller.

    pub fn gpib_control_ren(&self, mode: enums::gpib::RenMode) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viGpibControlREN(self.as_raw_ss(), mode as _))?;
        Ok(())
    }

    /// Tell the GPIB device at the specified address to become controller in charge (CIC).
    ///
    /// This operation passes controller in charge status to the device indicated by primAddr and secAddr, and then deasserts the ATN line. This operation assumes that the targeted device has controller capability. This operation is valid only on GPIB INTFC (interface) sessions.
    ///
    /// + `prim_addr`: Primary address of the GPIB device to which you want to pass control.
    ///
    /// + `sec_addr`: Secondary address of the targeted GPIB device. If the targeted device does not have a secondary address, this parameter should set as None or the value [VI_NO_SEC_ADDR](vs::VI_NO_SEC_ADDR).
    ///

    pub fn gpib_pass_control(
        &self,
        prim_addr: vs::ViUInt16,
        sec_addr: impl Into<Option<vs::ViUInt16>>,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viGpibPassControl(
            self.as_raw_ss(),
            prim_addr as _,
            sec_addr.into().unwrap_or(vs::VI_NO_SEC_ADDR as _) as _
        ))?;
        Ok(())
    }
    /// Pulse the interface clear line (IFC) for at least 100 microseconds.
    ///
    /// This operation asserts the IFC line and becomes controller in charge (CIC). The local board must be the system controller. This operation is valid only on GPIB INTFC (interface) sessions.
    ///

    pub fn gpib_send_ifc(&self) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viGpibSendIFC(self.as_raw_ss(),))?;
        Ok(())
    }
}
