use super::{FencedDescendants, ProcessScanner, scanner};
use crate::process::{ProcessIdentity, ScopedWaitableChild};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const SHELL: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const FOREGROUND_COMMAND: &str = "/bin/sleep 30; :";
const FOREGROUND_EXECUTABLE: &str = "/bin/sleep";
const SCAN_BUDGET: Duration = Duration::from_secs(5);
const SCAN_INTERVAL: Duration = Duration::from_millis(10);
const SYSTEM_CALL_FAILURE: libc::c_int = -1;
const NO_FORCE_CONTROLLING_TERMINAL: libc::c_int = 0;

#[tokio::test]
async fn isolated_scanner_accepts_real_linux_foreground_executable_identity() {
    let (master, child) = foreground_shell();
    let identity = child.captured_identity().unwrap();
    let expected = std::fs::canonicalize(FOREGROUND_EXECUTABLE).unwrap();
    let deadline = Instant::now() + SCAN_BUDGET;
    loop {
        let observation = scanner().scan(identity, deadline).await.unwrap();
        let FencedDescendants::Stable(processes) = observation.descendants else {
            panic!("private PTY shell identity changed during the scan");
        };
        if let Some(process) = processes
            .iter()
            .find(|process| std::path::Path::new(&process.executable) == expected)
        {
            assert_eq!(process.parent_pid, identity.pid);
            assert_eq!(process.process_group, process.foreground_group);
            assert_eq!(process.process_group, i32::try_from(identity.pid).unwrap());
            let expected_identity = ProcessIdentity::capture(process.pid).unwrap();
            assert_eq!(process.start_time, expected_identity.start_time);
            break;
        }
        assert!(
            Instant::now() < deadline,
            "foreground child was not observed"
        );
        tokio::time::sleep(SCAN_INTERVAL).await;
    }
    // Closing this private master hangs up only this fixture's foreground group.
    drop(master);
    child
        .terminate_and_wait_reaped(Instant::now() + SCAN_BUDGET)
        .await
        .unwrap();
}

fn foreground_shell() -> (OwnedFd, ScopedWaitableChild) {
    let mut master = SYSTEM_CALL_FAILURE;
    let mut slave = SYSTEM_CALL_FAILURE;
    // SAFETY: openpty initializes both fresh descriptors and retains no pointers.
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(result, libc::EXIT_SUCCESS);
    // SAFETY: successful openpty returned two owned descriptors.
    let (master, slave) = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    for descriptor in [&master, &slave] {
        // SAFETY: each descriptor is owned and remains open across both calls.
        let flags = unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_GETFD) };
        assert_ne!(flags, SYSTEM_CALL_FAILURE);
        let result = unsafe {
            libc::fcntl(
                descriptor.as_raw_fd(),
                libc::F_SETFD,
                flags | libc::FD_CLOEXEC,
            )
        };
        assert_ne!(result, SYSTEM_CALL_FAILURE);
    }
    let mut command = Command::new(SHELL);
    command
        .args([SHELL_COMMAND_ARGUMENT, FOREGROUND_COMMAND])
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave));
    // SAFETY: the child performs only async-signal-safe session and tty syscalls.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == SYSTEM_CALL_FAILURE
                || libc::ioctl(
                    libc::STDIN_FILENO,
                    libc::TIOCSCTTY,
                    NO_FORCE_CONTROLLING_TERMINAL,
                ) == SYSTEM_CALL_FAILURE
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child =
        ScopedWaitableChild::spawn_before_deadline(&mut command, Instant::now() + SCAN_BUDGET)
            .unwrap();
    drop(command);
    (master, child)
}
