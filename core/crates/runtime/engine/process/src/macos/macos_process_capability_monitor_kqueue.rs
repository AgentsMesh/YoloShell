use anyhow::{Context, Result, ensure};
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::Instant;

const INVALID_SYSTEM_RESULT: libc::c_int = -1;
const NO_EVENTS: libc::c_int = 0;
const SINGLE_EVENT: libc::c_int = 1;
const WAKE_EVENT_IDENT: usize = 1;
const QUEUE_CREATE_CONTEXT: &str = "create process capability event queue";
const QUEUE_REGISTER_CONTEXT: &str = "register process capability event";
const QUEUE_READ_CONTEXT: &str = "read process capability event";
const QUEUE_EVENT_DETAIL: &str = "process capability event is invalid";

pub(super) struct MonitorQueue {
    descriptor: OwnedFd,
    process_descriptor: libc::c_int,
    root_pid: u32,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum MonitorEvent {
    Descriptor,
    Deadline,
    RootExit,
    Wake,
}

impl MonitorQueue {
    pub(super) fn create(process_descriptor: libc::c_int, root_pid: u32) -> Result<Self> {
        let descriptor = super::super::super::macos_private_kqueue(QUEUE_CREATE_CONTEXT)?;
        let queue = Self {
            descriptor,
            process_descriptor,
            root_pid,
        };
        queue.register(read_event(process_descriptor))?;
        queue.register(root_event(root_pid))?;
        queue.register(wake_event(libc::EV_ADD | libc::EV_CLEAR, u32::default()))?;
        Ok(queue)
    }

    pub(super) fn trigger_wake(&self) -> Result<()> {
        self.register(wake_event(u16::default(), libc::NOTE_TRIGGER))
    }

    pub(super) fn next_event(&self, deadline: Option<Instant>) -> Result<MonitorEvent> {
        let timeout = deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .map(duration_timeout)
            .transpose()?;
        let Some(event) = self.read_event(timeout.as_ref())? else {
            if deadline.is_some()
                && let Some(event) = self.read_event(Some(&immediate_timeout()))?
            {
                return self.classify(event);
            }
            ensure!(deadline.is_some(), QUEUE_READ_CONTEXT);
            return Ok(MonitorEvent::Deadline);
        };
        self.classify(event)
    }

    fn read_event(&self, timeout: Option<&libc::timespec>) -> Result<Option<libc::kevent>> {
        let mut event = empty_event();
        let read = checked_system_result(
            unsafe {
                libc::kevent(
                    self.descriptor.as_raw_fd(),
                    std::ptr::null(),
                    NO_EVENTS,
                    std::ptr::addr_of_mut!(event),
                    SINGLE_EVENT,
                    timeout.map_or(std::ptr::null(), std::ptr::from_ref),
                )
            },
            QUEUE_READ_CONTEXT,
        )?;
        if read == NO_EVENTS {
            return Ok(None);
        }
        ensure!(read == SINGLE_EVENT, QUEUE_READ_CONTEXT);
        Ok(Some(event))
    }

    fn classify(&self, event: libc::kevent) -> Result<MonitorEvent> {
        ensure!(
            event.flags & libc::EV_ERROR == u16::default(),
            QUEUE_EVENT_DETAIL
        );
        match (event.filter, event.ident) {
            (libc::EVFILT_READ, ident) if ident == self.process_descriptor as usize => {
                Ok(MonitorEvent::Descriptor)
            }
            (libc::EVFILT_PROC, ident)
                if ident == self.root_pid as usize && event.fflags & libc::NOTE_EXIT != 0 =>
            {
                Ok(MonitorEvent::RootExit)
            }
            (libc::EVFILT_USER, WAKE_EVENT_IDENT) => Ok(MonitorEvent::Wake),
            _ => anyhow::bail!(QUEUE_EVENT_DETAIL),
        }
    }

    fn register(&self, event: libc::kevent) -> Result<()> {
        let changed = checked_system_result(
            unsafe {
                libc::kevent(
                    self.descriptor.as_raw_fd(),
                    std::ptr::addr_of!(event),
                    SINGLE_EVENT,
                    std::ptr::null_mut(),
                    NO_EVENTS,
                    std::ptr::null(),
                )
            },
            QUEUE_REGISTER_CONTEXT,
        )?;
        ensure!(changed == NO_EVENTS, QUEUE_REGISTER_CONTEXT);
        Ok(())
    }
}

fn duration_timeout(duration: std::time::Duration) -> Result<libc::timespec> {
    Ok(libc::timespec {
        tv_sec: duration.as_secs().try_into()?,
        tv_nsec: duration.subsec_nanos().into(),
    })
}

fn immediate_timeout() -> libc::timespec {
    libc::timespec {
        tv_sec: Default::default(),
        tv_nsec: Default::default(),
    }
}

fn checked_system_result(result: libc::c_int, context: &'static str) -> Result<libc::c_int> {
    if result == INVALID_SYSTEM_RESULT {
        return Err(std::io::Error::last_os_error()).context(context);
    }
    Ok(result)
}

fn read_event(descriptor: libc::c_int) -> libc::kevent {
    event(
        descriptor as usize,
        libc::EVFILT_READ,
        libc::EV_ADD | libc::EV_CLEAR | libc::EV_ONESHOT,
        u32::default(),
    )
}

fn root_event(pid: u32) -> libc::kevent {
    event(
        pid as usize,
        libc::EVFILT_PROC,
        libc::EV_ADD | libc::EV_CLEAR,
        libc::NOTE_EXIT,
    )
}

fn wake_event(flags: u16, fflags: u32) -> libc::kevent {
    event(WAKE_EVENT_IDENT, libc::EVFILT_USER, flags, fflags)
}

fn empty_event() -> libc::kevent {
    event(
        usize::default(),
        i16::default(),
        u16::default(),
        u32::default(),
    )
}

fn event(ident: usize, filter: i16, flags: u16, fflags: u32) -> libc::kevent {
    libc::kevent {
        ident,
        filter,
        flags,
        fflags,
        data: libc::intptr_t::default(),
        udata: std::ptr::null_mut(),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_monitor_kqueue_tests.rs"]
mod tests;
