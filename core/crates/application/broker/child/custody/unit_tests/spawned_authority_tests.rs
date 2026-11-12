use super::*;
use crate::fixture_constants as fixture;
use std::io;

const TEST_POLL_FAILURE: &str = "test poll failure";
const TEST_TERMINATE_FAILURE: &str = "test terminate failure";

#[test]
fn unknown_poll_failure_still_transfers_exact_child_after_termination() {
    let mut child = TestChild::unknown_poll_failure();

    let error = settle_test_child(&mut child).unwrap_err();

    assert!(error.to_string().starts_with(POLL_CHILD_CONTEXT));
    assert_eq!(
        child.operations,
        [
            fixture::TEST_POLL_OPERATION,
            fixture::TEST_TERMINATE_OPERATION,
        ]
    );
    assert_eq!(error.root_cause().to_string(), TEST_POLL_FAILURE);
}

#[test]
fn poll_and_termination_failures_are_both_visible_without_an_unsafe_wait() {
    let mut child = TestChild::poll_and_termination_failure();

    let error = settle_test_child(&mut child).unwrap_err();

    let message = error.to_string();
    assert!(message.contains(TEST_POLL_FAILURE));
    assert!(message.contains(TEST_TERMINATE_FAILURE));
    assert_eq!(
        child.operations,
        [
            fixture::TEST_POLL_OPERATION,
            fixture::TEST_TERMINATE_OPERATION,
        ]
    );
}

#[test]
fn echild_poll_is_typed_wait_authority_loss() {
    let mut child = TestChild::wait_authority_lost_on_poll();

    let error = settle_test_child(&mut child).unwrap_err();

    assert_wait_authority_loss(&error);
    assert_eq!(child.operations, [fixture::TEST_POLL_OPERATION]);
}

#[test]
fn termination_failure_returns_before_reap() {
    let mut child = TestChild::termination_failure();

    let error = settle_test_child(&mut child).unwrap_err();

    assert_eq!(error.root_cause().to_string(), TEST_TERMINATE_FAILURE);
    assert_eq!(
        child.operations,
        [
            fixture::TEST_POLL_OPERATION,
            fixture::TEST_TERMINATE_OPERATION,
        ]
    );
}

#[test]
fn running_child_is_not_synchronously_reaped_after_termination() {
    let mut child = TestChild::running();

    settle_test_child(&mut child).unwrap();

    assert_eq!(
        child.operations,
        [
            fixture::TEST_POLL_OPERATION,
            fixture::TEST_TERMINATE_OPERATION,
        ]
    );
}


fn assert_wait_authority_loss(error: &anyhow::Error) {
    let failure = error.downcast_ref::<ChildSettlementFailure>().unwrap();
    assert!(failure.wait_authority_lost());
    assert!(
        error
            .to_string()
            .contains(super::failure::WAIT_AUTHORITY_LOST_CONTEXT)
    );
}

struct TestChild {
    poll: Option<io::Result<bool>>,
    terminate: Option<io::Result<()>>,
    operations: Vec<&'static str>,
}

impl TestChild {
    fn unknown_poll_failure() -> Self {
        Self::new(
            Err(io::Error::other(TEST_POLL_FAILURE)),
            Some(Ok(())),
        )
    }

    fn poll_and_termination_failure() -> Self {
        Self::new(
            Err(io::Error::other(TEST_POLL_FAILURE)),
            Some(Err(io::Error::other(TEST_TERMINATE_FAILURE))),
        )
    }

    fn wait_authority_lost_on_poll() -> Self {
        Self::new(Err(echild()), None)
    }

    fn termination_failure() -> Self {
        Self::new(
            Ok(false),
            Some(Err(io::Error::other(TEST_TERMINATE_FAILURE))),
        )
    }

    fn running() -> Self {
        Self::new(Ok(false), Some(Ok(())))
    }

    fn new(
        poll: io::Result<bool>,
        terminate: Option<io::Result<()>>,
    ) -> Self {
        Self {
            poll: Some(poll),
            terminate,
            operations: Vec::new(),
        }
    }
}

impl ChildLifecycle for TestChild {
    fn poll_exited(&mut self) -> io::Result<bool> {
        self.operations.push(fixture::TEST_POLL_OPERATION);
        self.poll.take().unwrap()
    }

    fn terminate(&mut self) -> io::Result<()> {
        self.operations.push(fixture::TEST_TERMINATE_OPERATION);
        self.terminate.take().unwrap()
    }
}

fn echild() -> io::Error {
    io::Error::from_raw_os_error(libc::ECHILD)
}
