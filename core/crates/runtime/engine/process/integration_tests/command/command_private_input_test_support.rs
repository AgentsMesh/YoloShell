use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

const CHILD_READY_TIMEOUT_DETAIL: &str = "private input child did not publish readiness";
const CHILD_READY_IDENTITY_DETAIL: &str = "private input child published a different identity";

#[derive(Default)]
pub(super) struct ShortWriter {
    pub(super) output: Vec<u8>,
}

impl Write for ShortWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = bytes.len().min(1);
        self.output.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn wait_until_reaped(process_id: u32, timeout: Duration, poll: Duration) {
    let deadline = Instant::now() + timeout;
    while child_wait_authority_exists(process_id) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(poll);
    }
}

pub(super) fn wait_until_pid_published(
    path: &Path,
    expected: u32,
    timeout: Duration,
    poll: Duration,
) {
    let deadline = Instant::now() + timeout;
    loop {
        match std::fs::read_to_string(path) {
            Ok(value) => {
                if let Ok(observed) = value.parse::<u32>() {
                    assert_eq!(observed, expected, "{CHILD_READY_IDENTITY_DETAIL}");
                    return;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => panic!("{CHILD_READY_TIMEOUT_DETAIL}: {error}"),
        }
        assert!(Instant::now() < deadline, "{CHILD_READY_TIMEOUT_DETAIL}");
        std::thread::sleep(poll);
    }
}

fn child_wait_authority_exists(process_id: u32) -> bool {
    let mut status = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
    let options = libc::WEXITED | libc::WNOHANG | libc::WNOWAIT;
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            process_id as libc::id_t,
            status.as_mut_ptr(),
            options,
        )
    };
    if result == libc::c_int::default() {
        return true;
    }
    assert_eq!(
        io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
    false
}
