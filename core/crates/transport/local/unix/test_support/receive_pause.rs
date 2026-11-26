use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

#[path = "pipe.rs"]
mod pipe_fixture;
pub use pipe_fixture::{Pipe, pipe};
const RECEIVE_PAUSE_TIMEOUT: Duration = Duration::from_secs(2);
const SPAWN_GATE_PROBE_TIMEOUT: Duration = Duration::from_millis(25);
const RECEIVE_HOOK_ALREADY_INSTALLED: &str =
    "local Unix descriptor receive hook is already installed";
const RECEIVE_PAUSE_NOT_REACHED: &str = "local Unix descriptor receive hook was not reached";

static RECEIVE_HOOK: Mutex<Option<Arc<ReceivePause>>> = Mutex::new(None);
static RECEIVE_HOOK_INSTALL_GATE: Mutex<()> = Mutex::new(());

struct ReceivePause {
    socket: libc::c_int,
    state: Mutex<ReceivePauseState>,
    changed: Condvar,
}

#[derive(Default)]
struct ReceivePauseState {
    reached: bool,
    released: bool,
}

pub struct ReceivePauseGuard {
    pause: Arc<ReceivePause>,
    _install: MutexGuard<'static, ()>,
}

pub fn send_right(socket: libc::c_int, descriptor: libc::c_int, mut marker: u8) {
    let mut control = super::control_macos::AlignedControl::zeroed();
    let header = control.bytes.as_mut_ptr().cast::<libc::cmsghdr>();
    unsafe {
        (*header).cmsg_len = libc::CMSG_LEN(super::constants::DESCRIPTOR_BYTES as _);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        std::ptr::write_unaligned(libc::CMSG_DATA(header).cast(), descriptor);
    }
    let mut vector = libc::iovec {
        iov_base: std::ptr::addr_of_mut!(marker).cast(),
        iov_len: std::mem::size_of_val(&marker),
    };
    let message = libc::msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: libc::socklen_t::default(),
        msg_iov: std::ptr::addr_of_mut!(vector),
        msg_iovlen: super::constants::EXPECTED_IO_VECTOR_COUNT,
        msg_control: control.bytes.as_mut_ptr().cast(),
        msg_controllen: unsafe { libc::CMSG_SPACE(super::constants::DESCRIPTOR_BYTES as _) } as _,
        msg_flags: libc::c_int::default(),
    };
    assert_eq!(
        unsafe { libc::sendmsg(socket, &message, libc::MSG_NOSIGNAL) },
        super::constants::EXPECTED_MARKER_BYTES
    );
}

pub fn assert_reader_eof(reader: &OwnedFd) {
    let flags = unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_GETFL) };
    assert_ne!(flags, super::constants::FCNTL_FAILURE);
    assert_ne!(
        unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) },
        super::constants::FCNTL_FAILURE
    );
    let mut byte = u8::default();
    assert_eq!(
        unsafe {
            libc::read(
                reader.as_raw_fd(),
                std::ptr::addr_of_mut!(byte).cast(),
                std::mem::size_of_val(&byte),
            )
        },
        libc::ssize_t::default()
    );
}

pub fn install(socket: libc::c_int) -> ReceivePauseGuard {
    let install = RECEIVE_HOOK_INSTALL_GATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let pause = Arc::new(ReceivePause {
        socket,
        state: Mutex::new(ReceivePauseState::default()),
        changed: Condvar::new(),
    });
    let mut hook = RECEIVE_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(
        hook.replace(Arc::clone(&pause)).is_none(),
        "{RECEIVE_HOOK_ALREADY_INSTALLED}"
    );
    ReceivePauseGuard {
        pause,
        _install: install,
    }
}

pub fn spawn_gate_is_held() -> bool {
    let deadline = Instant::now() + SPAWN_GATE_PROBE_TIMEOUT;
    yolo_process_spawn::waitable_child_spawn_guard_before(deadline).is_err()
}

pub fn pause_after_recvmsg(socket: libc::c_int) {
    let pause = RECEIVE_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    if let Some(pause) = pause.filter(|pause| pause.socket == socket) {
        let mut state = pause
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.reached = true;
        pause.changed.notify_all();
        let _state = pause
            .changed
            .wait_while(state, |state| !state.released)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
    }
}

impl ReceivePauseGuard {
    pub fn wait_until_reached(&self) {
        let state = self
            .pause
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (state, timeout) = self
            .pause
            .changed
            .wait_timeout_while(state, RECEIVE_PAUSE_TIMEOUT, |state| !state.reached)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(
            state.reached && !timeout.timed_out(),
            "{RECEIVE_PAUSE_NOT_REACHED}"
        );
    }

    pub fn release(&self) {
        let mut state = self
            .pause
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.released = true;
        self.pause.changed.notify_all();
    }
}

impl Drop for ReceivePauseGuard {
    fn drop(&mut self) {
        self.release();
        RECEIVE_HOOK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
    }
}
