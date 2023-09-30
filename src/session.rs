use std::marker::PhantomData;

use visa_sys as vs;

/// Raw visa session.
pub type RawSs = vs::ViSession;

/// An owned visa session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedSs {
    s: RawSs,
}

/// A borrowed visa session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BorrowedSs<'b> {
    s: RawSs,
    _phantom: PhantomData<&'b RawSs>,
}

impl Drop for OwnedSs {
    fn drop(&mut self) {
        unsafe {
            vs::viClose(self.s);
        }
    }
}

impl BorrowedSs<'_> {
    /// # Safety
    ///
    /// The `ss` passed in must be a valid VISA session.

    pub unsafe fn borrow_raw(ss: RawSs) -> Self {
        Self {
            s: ss,
            _phantom: PhantomData,
        }
    }
}

/// A trait to extract the raw visa session from an underlying object.
pub trait AsRawSs {
    fn as_raw_ss(&self) -> RawSs;
}

/// A trait to express the ability to construct an object from a raw visa session.
pub trait FromRawSs {
    /// # Safety
    ///
    /// The `ss` passed in must be a valid VISA session.
    unsafe fn from_raw_ss(ss: RawSs) -> Self;
}

/// A trait to express the ability to consume an object and acquire ownership of its raw visa session.
pub trait IntoRawSs {
    fn into_raw_ss(self) -> RawSs;
}

/// A trait to borrow the visa session from an underlying object.
pub trait AsSs {
    fn as_ss(&self) -> BorrowedSs<'_>;
}

impl AsRawSs for BorrowedSs<'_> {
    fn as_raw_ss(&self) -> RawSs {
        self.s
    }
}

impl AsRawSs for OwnedSs {
    fn as_raw_ss(&self) -> RawSs {
        self.s
    }
}

impl IntoRawSs for OwnedSs {
    fn into_raw_ss(self) -> RawSs {
        let ss = self.s;
        std::mem::forget(self);
        ss
    }
}

impl FromRawSs for OwnedSs {
    unsafe fn from_raw_ss(s: RawSs) -> Self {
        Self { s }
    }
}

impl<T: AsSs> AsSs for &T {
    #[inline]
    fn as_ss(&self) -> BorrowedSs<'_> {
        T::as_ss(self)
    }
}

impl<T: AsSs> AsSs for &mut T {
    #[inline]
    fn as_ss(&self) -> BorrowedSs<'_> {
        T::as_ss(self)
    }
}

impl AsSs for BorrowedSs<'_> {
    #[inline]
    fn as_ss(&self) -> BorrowedSs<'_> {
        *self
    }
}

impl AsSs for OwnedSs {
    #[inline]
    fn as_ss(&self) -> BorrowedSs<'_> {
        unsafe { BorrowedSs::borrow_raw(self.s) }
    }
}
