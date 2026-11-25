use super::process_escape_support::{
    UnrelatedProcess, assert_gone, read_identity, read_ready_pid, write_all, write_identity,
};
use super::process_supervision_tests::Fixture;
use super::test_constants::{
    CONTENDED_DOUBLE_FORK_ESCAPE_DESCRIPTION, CONTENDED_DOUBLE_FORK_ESCAPE_FIXTURE,
    DOUBLE_FORK_ESCAPE_DESCRIPTION, DOUBLE_FORK_ESCAPE_FIXTURE, ESCAPE_CONTENTION_CASES,
    ESCAPE_STRESS_CASES, ESCAPED_CLOSED_DESCENDANT_FIXTURE, ESCAPED_CLOSED_PIPE_DESCRIPTION,
    ESCAPED_DESCENDANT_FIXTURE, ESCAPED_DESCENDANT_SECONDS, ESCAPED_PID_DESCRIPTOR,
    ESCAPED_PIPE_HOLDER_DESCRIPTION, PROCESS_COMPLETION_LIMIT_SECONDS, TEST_EXACT_ARGUMENT,
    TEST_IGNORED_ARGUMENT,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use yolo_process::{ProcessIdentity, ProcessIdentityObservation};

const FIXTURE_SETUP_EXIT: libc::c_int = 2;

#[test]
fn escaped_descendant_holding_pipes_is_cleaned_before_success() {
    let started = Instant::now();
    let identity = execute_escape(ESCAPED_DESCENDANT_FIXTURE, ESCAPED_PIPE_HOLDER_DESCRIPTION);
    assert_gone(identity);
    assert!(started.elapsed() < Duration::from_secs(PROCESS_COMPLETION_LIMIT_SECONDS));
}

#[test]
fn setsid_descendant_closing_pipes_is_cleaned_before_success() {
    assert_gone(execute_escape(
        ESCAPED_CLOSED_DESCENDANT_FIXTURE,
        ESCAPED_CLOSED_PIPE_DESCRIPTION,
    ));
}

#[test]
fn immediate_double_fork_escape_is_cleaned_without_signaling_unrelated_process() {
    let unrelated = UnrelatedProcess::spawn();
    for _ in 0..ESCAPE_STRESS_CASES {
        assert_gone(execute_escape(
            DOUBLE_FORK_ESCAPE_FIXTURE,
            DOUBLE_FORK_ESCAPE_DESCRIPTION,
        ));
        assert_eq!(
            unrelated.identity.observe(),
            ProcessIdentityObservation::Matching
        );
    }
}

#[test]
fn term_resistant_double_fork_escapes_settle_under_process_contention() {
    let unrelated = UnrelatedProcess::spawn();
    let identities = std::thread::scope(|scope| {
        (0..ESCAPE_CONTENTION_CASES)
            .map(|_| {
                scope.spawn(|| {
                    execute_escape(
                        CONTENDED_DOUBLE_FORK_ESCAPE_FIXTURE,
                        CONTENDED_DOUBLE_FORK_ESCAPE_DESCRIPTION,
                    )
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    identities.into_iter().for_each(assert_gone);
    assert_eq!(
        unrelated.identity.observe(),
        ProcessIdentityObservation::Matching
    );
}

#[test]
#[ignore]
fn escaped_pipe_holder_fixture() {
    run_fixture(EscapeShape::SingleFork, false, false);
}

#[test]
#[ignore]
fn escaped_closed_pipe_fixture() {
    run_fixture(EscapeShape::SingleFork, true, false);
}

#[test]
#[ignore]
fn immediate_double_fork_fixture() {
    run_fixture(EscapeShape::DoubleFork, true, false);
}

#[test]
#[ignore]
fn term_resistant_double_fork_fixture() {
    run_fixture(EscapeShape::DoubleFork, true, true);
}

fn execute_escape(fixture_name: &str, description: &str) -> ProcessIdentity {
    let fixture = Fixture::new();
    let marker = Arc::new(tempfile::tempfile().unwrap());
    let output = fixture
        .command(
            std::env::current_exe().unwrap(),
            fixture_arguments(fixture_name),
            description,
        )
        .inherit_as(Arc::clone(&marker), ESCAPED_PID_DESCRIPTOR)
        .execute()
        .unwrap();
    assert!(output.status.success());
    read_identity(&marker)
}

fn run_fixture(shape: EscapeShape, close_output: bool, ignore_term: bool) {
    let mut readiness = [0; 2];
    assert_eq!(unsafe { libc::pipe(readiness.as_mut_ptr()) }, 0);
    let intermediate = unsafe { libc::fork() };
    assert!(intermediate >= 0);
    if intermediate == 0 {
        launch_leaf(readiness, shape, close_output, ignore_term);
    }
    unsafe { libc::close(readiness[1]) };
    let pid = read_ready_pid(readiness[0]);
    unsafe { libc::close(readiness[0]) };
    write_identity(ProcessIdentity::capture(pid).unwrap());
}

fn launch_leaf(
    readiness: [libc::c_int; 2],
    shape: EscapeShape,
    close_output: bool,
    ignore_term: bool,
) -> ! {
    unsafe { libc::close(readiness[0]) };
    if shape == EscapeShape::DoubleFork {
        let leaf = unsafe { libc::fork() };
        if leaf < 0 {
            unsafe { libc::_exit(FIXTURE_SETUP_EXIT) };
        }
        if leaf > 0 {
            unsafe { libc::_exit(0) };
        }
    }
    if unsafe { libc::setsid() } < 0 {
        unsafe { libc::_exit(FIXTURE_SETUP_EXIT) };
    }
    if ignore_term && unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) } == libc::SIG_ERR {
        unsafe { libc::_exit(FIXTURE_SETUP_EXIT) };
    }
    if close_output {
        unsafe {
            libc::close(libc::STDOUT_FILENO);
            libc::close(libc::STDERR_FILENO);
        }
    }
    let pid = u32::try_from(unsafe { libc::getpid() }).unwrap();
    write_all(readiness[1], &pid.to_ne_bytes());
    unsafe {
        libc::close(readiness[1]);
        libc::sleep(ESCAPED_DESCENDANT_SECONDS);
        libc::_exit(0);
    }
}

fn fixture_arguments(name: &str) -> Vec<String> {
    vec![
        TEST_IGNORED_ARGUMENT.to_owned(),
        TEST_EXACT_ARGUMENT.to_owned(),
        name.to_owned(),
    ]
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum EscapeShape {
    SingleFork,
    DoubleFork,
}
