use std::cell::Cell;
use std::io;

const NESTED_HOOK: &str = "broker settlement termination hook already installed";

thread_local! {
    static TERMINATION_FAILURE: Cell<Option<&'static str>> = const { Cell::new(None) };
}

pub(crate) struct TerminationFailureGuard;

pub(crate) fn install_termination_failure(detail: &'static str) -> TerminationFailureGuard {
    TERMINATION_FAILURE.with(|failure| {
        assert!(failure.replace(Some(detail)).is_none(), "{NESTED_HOOK}");
    });
    TerminationFailureGuard
}

pub(super) fn take_termination_failure() -> Option<io::Error> {
    TERMINATION_FAILURE.with(|failure| failure.take().map(io::Error::other))
}

impl Drop for TerminationFailureGuard {
    fn drop(&mut self) {
        TERMINATION_FAILURE.with(|failure| failure.set(None));
    }
}
