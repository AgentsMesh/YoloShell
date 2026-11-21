use super::child_reaper::worker::{ReapChild, reap_retained};
use super::child_reaper::{RetainedChild, TerminationPolicy};
use super::*;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

const QUICK_EXIT_EXECUTABLE: &str = "/bin/sh";
const QUICK_EXIT_SCRIPT: &str = "while test -e \"$1\"; do :; done";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const SHELL_ARGUMENT_ZERO: &str = "yolo-reaper-gate";
const EXIT_GATE_FILE: &str = "exit-gate";
const EXIT_GATE_CONTENT: &[u8] = b"held";
const QUICK_EXIT_COUNT: usize = 16;
const REAP_DEADLINE: Duration = Duration::from_secs(2);
const REAP_POLL_INTERVAL: Duration = Duration::from_millis(10);
const WAITID_OBSERVE_OPTIONS: libc::c_int = libc::WEXITED | libc::WNOHANG | libc::WNOWAIT;
const WAITID_SUCCESS: libc::c_int = 0;
const WAITPID_BLOCKING_OPTIONS: libc::c_int = 0;
const WAITPID_SYSCALL_FAILURE: libc::pid_t = -1;
const TEST_CHILD_ID_BASE: u32 = 7_000;
const TEST_REAP_FAILURE: &str = "test persistent reap failure";
const TEST_TERMINATION_FAILURE: &str = "test termination failure";
const ALREADY_REAPED_OFFSET: u32 = 0;
const RETRY_OFFSET: u32 = 1;
const UNRECOVERABLE_OFFSET: u32 = 2;
const EXITED_OFFSET: u32 = 3;
const RUNNING_OFFSET: u32 = 4;

#[test]
fn detached_child_handoffs_reap_repeated_quick_exits() {
    let directory = tempfile::tempdir().unwrap();
    let exit_gate = directory.path().join(EXIT_GATE_FILE);
    std::fs::write(&exit_gate, EXIT_GATE_CONTENT).unwrap();
    let arguments = [
        OsString::from(SHELL_COMMAND_ARGUMENT),
        OsString::from(QUICK_EXIT_SCRIPT),
        OsString::from(SHELL_ARGUMENT_ZERO),
        exit_gate.as_os_str().to_owned(),
    ];
    let mut identities = Vec::with_capacity(QUICK_EXIT_COUNT);
    for _ in 0..QUICK_EXIT_COUNT {
        let child = spawn_detached(
            Path::new(QUICK_EXIT_EXECUTABLE),
            &arguments,
            DetachedStandardError::Null,
        )
        .unwrap();
        identities.push(ProcessIdentity::capture(child.id()).unwrap());
        handoff_detached_child(child).unwrap();
    }
    std::fs::remove_file(exit_gate).unwrap();

    let deadline = Instant::now() + REAP_DEADLINE;
    while identities.iter().any(child_is_waitable) && Instant::now() < deadline {
        std::thread::sleep(REAP_POLL_INTERVAL);
    }

    for identity in identities {
        assert!(!child_is_waitable(&identity));
        assert!(!durable_locator_is_present(&identity));
        let mut status = std::mem::MaybeUninit::<libc::c_int>::uninit();
        let waited = unsafe {
            libc::waitpid(
                identity.pid as libc::pid_t,
                status.as_mut_ptr(),
                WAITPID_BLOCKING_OPTIONS,
            )
        };
        assert_eq!(waited, WAITPID_SYSCALL_FAILURE);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
}

fn child_is_waitable(identity: &ProcessIdentity) -> bool {
    let mut status = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            identity.pid as libc::id_t,
            status.as_mut_ptr(),
            WAITID_OBSERVE_OPTIONS,
        )
    };
    if result == WAITID_SUCCESS {
        return true;
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::Interrupted {
        return true;
    }
    assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
    false
}

fn durable_locator_is_present(identity: &ProcessIdentity) -> bool {
    process_start_time(identity.pid).ok() == Some(identity.start_time)
}

struct TestChild {
    process_id: u32,
    reaps: VecDeque<io::Result<bool>>,
    terminations: VecDeque<io::Result<()>>,
}

impl ReapChild for TestChild {
    fn process_id(&self) -> u32 {
        self.process_id
    }

    fn try_reap(&mut self) -> io::Result<bool> {
        self.reaps.pop_front().unwrap()
    }

    fn request_termination(&mut self) -> io::Result<()> {
        self.terminations.pop_front().unwrap()
    }
}

#[test]
fn all_unobserved_children_remain_in_exact_custody() {
    let mut children = vec![
        retained(
            ALREADY_REAPED_OFFSET,
            Err(io::Error::from_raw_os_error(libc::ECHILD)),
        ),
        retained(RETRY_OFFSET, Err(io::ErrorKind::Interrupted.into())),
        retained(
            UNRECOVERABLE_OFFSET,
            Err(io::Error::other(TEST_REAP_FAILURE)),
        ),
        retained(EXITED_OFFSET, Ok(true)),
        retained(RUNNING_OFFSET, Ok(false)),
    ];

    assert_eq!(reap_retained(&mut children), 2);

    let mut retained = children
        .iter()
        .map(|child| child.child.process_id())
        .collect::<Vec<_>>();
    retained.sort_unstable();
    assert_eq!(
        retained,
        [
            TEST_CHILD_ID_BASE + RETRY_OFFSET,
            TEST_CHILD_ID_BASE + UNRECOVERABLE_OFFSET,
            TEST_CHILD_ID_BASE + RUNNING_OFFSET
        ]
    );
    assert!(is_child_already_reaped(&io::Error::from_raw_os_error(
        libc::ECHILD
    )));
}

#[test]
fn termination_policy_retries_a_failed_kill_without_dropping_custody() {
    let mut children = vec![RetainedChild {
        child: TestChild {
            process_id: TEST_CHILD_ID_BASE,
            reaps: VecDeque::from([Ok(false), Ok(false), Ok(true)]),
            terminations: VecDeque::from([Err(io::Error::other(TEST_TERMINATION_FAILURE)), Ok(())]),
        },
        policy: TerminationPolicy::Terminate,
    }];

    assert_eq!(reap_retained(&mut children), 0);
    assert_eq!(children[0].policy, TerminationPolicy::Terminate);
    assert_eq!(reap_retained(&mut children), 0);
    assert_eq!(children[0].policy, TerminationPolicy::NaturalExit);
    assert_eq!(reap_retained(&mut children), 1);
    assert!(children.is_empty());
}

fn retained(offset: u32, result: io::Result<bool>) -> RetainedChild<TestChild> {
    RetainedChild {
        child: TestChild {
            process_id: TEST_CHILD_ID_BASE + offset,
            reaps: VecDeque::from([result]),
            terminations: VecDeque::new(),
        },
        policy: TerminationPolicy::NaturalExit,
    }
}
