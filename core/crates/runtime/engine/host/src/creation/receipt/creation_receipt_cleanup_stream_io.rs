unsafe fn receipt_fdopendir(descriptor: libc::c_int) -> *mut libc::DIR {
    #[cfg(test)]
    if take_receipt_stream_fault(ReceiptStreamFault::Open) {
        unsafe { *receipt_errno_location() = libc::EIO };
        return std::ptr::null_mut();
    }
    unsafe { libc::fdopendir(descriptor) }
}

unsafe fn receipt_readdir(stream: *mut libc::DIR) -> *mut libc::dirent {
    #[cfg(test)]
    if take_receipt_stream_fault(ReceiptStreamFault::Read) {
        unsafe { *receipt_errno_location() = libc::EIO };
        return std::ptr::null_mut();
    }
    unsafe { libc::readdir(stream) }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReceiptStreamFault {
    Open,
    Read,
}

#[cfg(test)]
thread_local! {
    static RECEIPT_STREAM_FAULT: std::cell::Cell<Option<ReceiptStreamFault>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) struct ReceiptStreamFaultGuard;

#[cfg(test)]
pub(crate) fn install_receipt_stream_fault(fault: ReceiptStreamFault) -> ReceiptStreamFaultGuard {
    RECEIPT_STREAM_FAULT.with(|slot| {
        assert!(slot.replace(Some(fault)).is_none());
    });
    ReceiptStreamFaultGuard
}

#[cfg(test)]
pub(crate) fn take_receipt_stream_fault(expected: ReceiptStreamFault) -> bool {
    RECEIPT_STREAM_FAULT.with(|slot| {
        let matches = slot.get() == Some(expected);
        if matches {
            slot.set(None);
        }
        matches
    })
}

#[cfg(test)]
impl Drop for ReceiptStreamFaultGuard {
    fn drop(&mut self) {
        RECEIPT_STREAM_FAULT.with(|slot| slot.set(None));
    }
}
