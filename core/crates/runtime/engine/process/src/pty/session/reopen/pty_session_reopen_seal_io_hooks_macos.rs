#[cfg(any(test, feature = "runtime-component-tests"))]
use anyhow::Context;
use anyhow::Result;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::process::pty_session_proof) enum TestIoOperation {
    OpenSlave,
    ChangeMode,
    ValidateSeal,
    FinalizeQuarantine,
}

#[cfg(any(test, feature = "runtime-component-tests"))]
thread_local! {
    static NEXT_FAILURE: std::cell::Cell<Option<TestIoOperation>> = const { std::cell::Cell::new(None) };
    static REPEATED_FAILURES: std::cell::RefCell<Vec<TestIoOperation>> = const {
        std::cell::RefCell::new(Vec::new())
    };
    static REPEATED_FAILURES_ARMED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::process::pty_session_proof) struct RepeatedFailureGuard;

#[cfg(any(test, feature = "runtime-component-tests"))]
impl Drop for RepeatedFailureGuard {
    fn drop(&mut self) {
        NEXT_FAILURE.with(|failure| failure.set(None));
        REPEATED_FAILURES.with(|failures| failures.borrow_mut().clear());
        REPEATED_FAILURES_ARMED.with(|armed| armed.set(false));
    }
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::process::pty_session_proof) fn fail_next(operation: TestIoOperation) {
    NEXT_FAILURE.with(|failure| failure.set(Some(operation)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::process::pty_session_proof) fn fail_repeated_after_next(
    operations: &[TestIoOperation],
) -> RepeatedFailureGuard {
    NEXT_FAILURE.with(|failure| assert!(failure.get().is_some()));
    REPEATED_FAILURES.with(|failures| {
        let mut failures = failures.borrow_mut();
        assert!(failures.is_empty());
        failures.extend_from_slice(operations);
    });
    RepeatedFailureGuard
}

pub(super) fn injected_failure(operation: TestIoOperation) -> Result<()> {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    if failure_is_armed(operation) {
        NEXT_FAILURE.with(|failure| {
            if failure.get() == Some(operation) {
                failure.set(None);
            }
        });
        let context = match operation {
            TestIoOperation::OpenSlave => super::SLAVE_OPEN_CONTEXT,
            TestIoOperation::ChangeMode
            | TestIoOperation::ValidateSeal
            | TestIoOperation::FinalizeQuarantine => super::SLAVE_CHMOD_CONTEXT,
        };
        return Err(std::io::Error::from_raw_os_error(libc::EPERM)).context(context);
    }
    let _ = operation;
    Ok(())
}

#[cfg(any(test, feature = "runtime-component-tests"))]
fn failure_is_armed(operation: TestIoOperation) -> bool {
    let single = NEXT_FAILURE.with(|failure| failure.get() == Some(operation));
    if single {
        REPEATED_FAILURES_ARMED.with(|armed| armed.set(true));
    }
    single
        || REPEATED_FAILURES_ARMED.with(|armed| armed.get())
            && REPEATED_FAILURES.with(|failures| failures.borrow().contains(&operation))
}
