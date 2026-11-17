use std::os::fd::RawFd;

const INVALID_DESCRIPTOR: libc::c_int = -1;
const WAIT_FOREVER: libc::c_int = -1;
const POLL_ENDPOINT_COUNT: libc::nfds_t = 2;
const CONTROL_ENDPOINT_INDEX: usize = 0;
const WORKER_ENDPOINT_INDEX: usize = 1;
const SINGLE_MESSAGE_BYTE: usize = 1;

pub enum GuardianInput {
    Command(u8),
    WorkerLost,
    ControlLost,
    Fault,
}

pub fn next_input(channel: RawFd, liveness: Option<RawFd>) -> GuardianInput {
    let mut endpoints = [
        poll_endpoint(channel, libc::POLLIN),
        poll_endpoint(liveness.unwrap_or(INVALID_DESCRIPTOR), libc::POLLIN),
    ];
    loop {
        let result =
            unsafe { libc::poll(endpoints.as_mut_ptr(), POLL_ENDPOINT_COUNT, WAIT_FOREVER) };
        if result < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if result <= 0 {
            return GuardianInput::Fault;
        }
        let worker_events = endpoints[WORKER_ENDPOINT_INDEX].revents;
        if worker_events & (libc::POLLIN | libc::POLLHUP) != 0 {
            return worker_event(endpoints[WORKER_ENDPOINT_INDEX].fd);
        }
        if endpoints[CONTROL_ENDPOINT_INDEX].revents & (libc::POLLIN | libc::POLLHUP) != 0 {
            return receive(channel);
        }
    }
}

fn worker_event(descriptor: RawFd) -> GuardianInput {
    let mut byte = u8::default();
    let count = unsafe {
        libc::read(
            descriptor,
            (&mut byte as *mut u8).cast(),
            SINGLE_MESSAGE_BYTE,
        )
    };
    if count == 0 {
        GuardianInput::WorkerLost
    } else {
        GuardianInput::Fault
    }
}

fn receive(channel: RawFd) -> GuardianInput {
    let mut command = u8::default();
    let count = unsafe {
        libc::recv(
            channel,
            (&mut command as *mut u8).cast(),
            SINGLE_MESSAGE_BYTE,
            0,
        )
    };
    if count == SINGLE_MESSAGE_BYTE as isize {
        GuardianInput::Command(command)
    } else {
        GuardianInput::ControlLost
    }
}

fn poll_endpoint(descriptor: RawFd, events: libc::c_short) -> libc::pollfd {
    libc::pollfd {
        fd: descriptor,
        events,
        revents: libc::c_short::default(),
    }
}
