use crate::pty_custody::CustodyHolderIdentity;
use anyhow::{Context, Result, ensure};
use std::os::fd::{AsRawFd, OwnedFd, RawFd};

const QUEUE_CONTEXT: &str = "create Guardian idle observation queue";
const WATCH_CONTEXT: &str = "register Guardian idle observation";
const READ_CONTEXT: &str = "read Guardian idle observation";
const PROCESS_CHANGED: &str = "Guardian observed process changed while arming";
const NO_EVENTS: libc::c_int = 0;
const SINGLE_EVENT: libc::c_int = 1;
const SYSTEM_ERROR: libc::c_int = -1;
const DIRECTORY_CHANGES: u32 =
    libc::NOTE_DELETE | libc::NOTE_RENAME | libc::NOTE_REVOKE | libc::NOTE_ATTRIB;
const FILE_CHANGES: u32 =
    DIRECTORY_CHANGES | libc::NOTE_WRITE | libc::NOTE_EXTEND | libc::NOTE_LINK;
const PROCESS_CHANGES: u32 = libc::NOTE_EXIT | libc::NOTE_EXEC;

pub(super) struct ObservationQueue(OwnedFd);

impl ObservationQueue {
    pub(super) fn create() -> Result<Self> {
        crate::process::macos_private_kqueue(QUEUE_CONTEXT).map(Self)
    }

    pub(super) fn watch_vnode(&self, descriptor: RawFd, directory: bool) -> Result<()> {
        self.register(event(
            descriptor.try_into()?,
            libc::EVFILT_VNODE,
            if directory {
                DIRECTORY_CHANGES
            } else {
                FILE_CHANGES
            },
        ))
    }

    pub(super) fn watch_process(&self, process: CustodyHolderIdentity) -> Result<()> {
        ensure!(process.matches(), PROCESS_CHANGED);
        self.register(event(
            process.pid.try_into()?,
            libc::EVFILT_PROC,
            PROCESS_CHANGES,
        ))?;
        ensure!(process.matches(), PROCESS_CHANGED);
        Ok(())
    }

    pub(super) fn unchanged(&self) -> Result<bool> {
        let mut observed = event(usize::default(), i16::default(), u32::default());
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let count = unsafe {
            libc::kevent(
                self.0.as_raw_fd(),
                std::ptr::null(),
                NO_EVENTS,
                std::ptr::addr_of_mut!(observed),
                SINGLE_EVENT,
                &timeout,
            )
        };
        checked(count, READ_CONTEXT)?;
        // Every event, including EV_ERROR and unfamiliar flags, forces full validation.
        Ok(count == NO_EVENTS)
    }

    fn register(&self, event: libc::kevent) -> Result<()> {
        let count = unsafe {
            libc::kevent(
                self.0.as_raw_fd(),
                std::ptr::addr_of!(event),
                SINGLE_EVENT,
                std::ptr::null_mut(),
                NO_EVENTS,
                std::ptr::null(),
            )
        };
        checked(count, WATCH_CONTEXT)?;
        ensure!(count == NO_EVENTS, WATCH_CONTEXT);
        Ok(())
    }
}

fn event(ident: usize, filter: i16, fflags: u32) -> libc::kevent {
    libc::kevent {
        ident,
        filter,
        flags: libc::EV_ADD | libc::EV_CLEAR,
        fflags,
        data: Default::default(),
        udata: std::ptr::null_mut(),
    }
}

fn checked(result: libc::c_int, context: &'static str) -> Result<()> {
    if result == SYSTEM_ERROR {
        return Err(std::io::Error::last_os_error()).context(context);
    }
    Ok(())
}
