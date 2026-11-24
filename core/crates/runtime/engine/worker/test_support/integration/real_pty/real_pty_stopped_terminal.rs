use std::os::fd::{AsRawFd, FromRawFd};

const STOPPED_OUTPUT_FD_MINIMUM: libc::c_int = 0;
const STOPPED_OUTPUT_OPERATION_SUCCESS: libc::c_int = 0;
const STOPPED_OUTPUT_FCNTL_FAILURE: libc::c_int = -1;
const STOPPED_OUTPUT_WRITE_FAILURE: libc::ssize_t = -1;
const STOPPED_OUTPUT_CONTROL_BYTES: libc::ssize_t = 1;
const STOPPED_OUTPUT_WARM_BYTES: u64 = STOPPED_OUTPUT_PAYLOAD_BYTES as u64;
const STOPPED_OUTPUT_SAMPLE_WINDOW: Duration = Duration::from_millis(75);
const STOPPED_OUTPUT_PROGRESS_TIMEOUT: Duration = Duration::from_secs(15);
const STOPPED_OUTPUT_STABLE_WINDOWS: usize = 4;
const STOPPED_OUTPUT_MASTER_FD_DETAIL: &str = "stopped-output PTY master has no native fd";
const STOPPED_OUTPUT_DUPLICATE_DETAIL: &str = "duplicate stopped-output PTY master";
const STOPPED_OUTPUT_GETATTR_DETAIL: &str = "read stopped-output terminal attributes";
const STOPPED_OUTPUT_SETATTR_DETAIL: &str = "enable stopped-output software flow control";
const STOPPED_OUTPUT_CTRL_S_DETAIL: &str = "inject stopped-output VSTOP byte";
const STOPPED_OUTPUT_IOCTL_DETAIL: &str = "apply TIOCSTOP to stopped-output PTY";
const STOPPED_OUTPUT_WARMUP_DETAIL: &str = "writer did not reach warmup progress";
const STOPPED_OUTPUT_WARMUP_DIAGNOSTIC: &str = "stopped-output warmup diagnostic";
const STOPPED_OUTPUT_ADVANCE_DETAIL: &str = "writer did not advance before output stop";
const STOPPED_OUTPUT_BACKPRESSURE_DETAIL: &str = "writer progress did not become backpressured";

struct StoppedOutputGuard {
    master: std::fs::File,
    original: libc::termios,
    mode: StoppedOutputMode,
}

impl StoppedOutputGuard {
    fn prepare(master: &dyn MasterPty, mode: StoppedOutputMode) -> Result<Self> {
        let fd = master
            .as_raw_fd()
            .context(STOPPED_OUTPUT_MASTER_FD_DETAIL)?;
        let duplicate =
            unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, STOPPED_OUTPUT_FD_MINIMUM) };
        if duplicate == STOPPED_OUTPUT_FCNTL_FAILURE {
            return Err(std::io::Error::last_os_error()).context(STOPPED_OUTPUT_DUPLICATE_DETAIL);
        }
        let master = unsafe { std::fs::File::from_raw_fd(duplicate) };
        let original = terminal_attributes(master.as_raw_fd())?;
        let guard = Self {
            master,
            original,
            mode,
        };
        if mode == StoppedOutputMode::CtrlS {
            guard.enable_ctrl_s()?;
        }
        Ok(guard)
    }

    fn stop(&self) -> Result<()> {
        match self.mode {
            StoppedOutputMode::CtrlS => self.stop_with_ctrl_s(),
            StoppedOutputMode::Tiocstop => ioctl_without_argument(
                self.master.as_raw_fd(),
                libc::TIOCSTOP.into(),
                STOPPED_OUTPUT_IOCTL_DETAIL,
            ),
        }
    }

    fn enable_ctrl_s(&self) -> Result<()> {
        let mut attributes = self.original;
        attributes.c_iflag |= libc::IXON;
        if unsafe { libc::tcsetattr(self.master.as_raw_fd(), libc::TCSANOW, &attributes) }
            != STOPPED_OUTPUT_OPERATION_SUCCESS
        {
            return Err(std::io::Error::last_os_error()).context(STOPPED_OUTPUT_SETATTR_DETAIL);
        }
        Ok(())
    }

    fn stop_with_ctrl_s(&self) -> Result<()> {
        write_control_byte(self.master.as_raw_fd(), self.original.c_cc[libc::VSTOP])
    }
}

impl Drop for StoppedOutputGuard {
    fn drop(&mut self) {
        let fd = self.master.as_raw_fd();
        let _ = unsafe { libc::tcflow(fd, libc::TCOON) };
        let _ = unsafe { libc::ioctl(fd, libc::TIOCSTART.into()) };
        let _ = unsafe { libc::tcsetattr(fd, libc::TCSANOW, &self.original) };
    }
}

fn terminal_attributes(fd: libc::c_int) -> Result<libc::termios> {
    let mut attributes = std::mem::MaybeUninit::<libc::termios>::uninit();
    if unsafe { libc::tcgetattr(fd, attributes.as_mut_ptr()) } != STOPPED_OUTPUT_OPERATION_SUCCESS {
        return Err(std::io::Error::last_os_error()).context(STOPPED_OUTPUT_GETATTR_DETAIL);
    }
    Ok(unsafe { attributes.assume_init() })
}

fn ioctl_without_argument(
    fd: libc::c_int,
    operation: libc::c_ulong,
    detail: &'static str,
) -> Result<()> {
    if unsafe { libc::ioctl(fd, operation) } == STOPPED_OUTPUT_OPERATION_SUCCESS {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context(detail)
    }
}

fn write_control_byte(fd: libc::c_int, byte: libc::cc_t) -> Result<()> {
    let result = unsafe {
        libc::write(
            fd,
            std::ptr::from_ref(&byte).cast(),
            STOPPED_OUTPUT_CONTROL_BYTES as usize,
        )
    };
    if result == STOPPED_OUTPUT_CONTROL_BYTES {
        return Ok(());
    }
    if result == STOPPED_OUTPUT_WRITE_FAILURE {
        return Err(std::io::Error::last_os_error()).context(STOPPED_OUTPUT_CTRL_S_DETAIL);
    }
    anyhow::bail!(STOPPED_OUTPUT_CTRL_S_DETAIL)
}

fn wait_for_stopped_output_warmup(fixture: &Fixture, path: &Path) {
    let deadline = Instant::now() + STOPPED_OUTPUT_PROGRESS_TIMEOUT;
    loop {
        if read_stopped_output_progress(path).unwrap() >= STOPPED_OUTPUT_WARM_BYTES {
            return;
        }
        if Instant::now() >= deadline {
            let terminal = lock_recover(&fixture.state.terminal).screen().contents();
            panic!(
                "{STOPPED_OUTPUT_WARMUP_DETAIL}; {STOPPED_OUTPUT_WARMUP_DIAGNOSTIC}: root_live={}, terminal={terminal:?}",
                fixture_root_identity(fixture).matches()
            );
        }
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn prove_stopped_output_advance(path: &Path) {
    let initial = read_stopped_output_progress(path).unwrap();
    let deadline = Instant::now() + STOPPED_OUTPUT_PROGRESS_TIMEOUT;
    loop {
        std::thread::sleep(STOPPED_OUTPUT_SAMPLE_WINDOW);
        if read_stopped_output_progress(path).unwrap() > initial {
            return;
        }
        assert!(Instant::now() < deadline, "{STOPPED_OUTPUT_ADVANCE_DETAIL}");
    }
}

fn prove_stopped_output_backpressure(path: &Path) {
    let deadline = Instant::now() + STOPPED_OUTPUT_PROGRESS_TIMEOUT;
    let mut previous = read_stopped_output_progress(path).unwrap();
    let mut stable = 0_usize;
    while stable < STOPPED_OUTPUT_STABLE_WINDOWS {
        std::thread::sleep(STOPPED_OUTPUT_SAMPLE_WINDOW);
        let current = read_stopped_output_progress(path).unwrap();
        stable = if current == previous { stable + 1 } else { 0 };
        previous = current;
        assert!(
            Instant::now() < deadline,
            "{STOPPED_OUTPUT_BACKPRESSURE_DETAIL}"
        );
    }
}
