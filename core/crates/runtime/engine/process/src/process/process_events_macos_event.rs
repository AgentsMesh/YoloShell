pub(super) fn process_event(pid: u32, flags: u16, fflags: u32) -> libc::kevent {
    libc::kevent {
        ident: pid as usize,
        filter: libc::EVFILT_PROC,
        flags,
        fflags,
        data: libc::intptr_t::default(),
        udata: std::ptr::null_mut(),
    }
}

pub(super) fn empty_event() -> libc::kevent {
    process_event(u32::default(), u16::default(), u32::default())
}
