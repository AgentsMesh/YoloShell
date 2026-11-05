use anyhow::Error;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(super) const ERROR_DETAIL_FALLBACK: &str = "PTY reopen-seal cleanup error formatting panicked";

pub(super) fn remember_failure(slot: &mut Option<Error>, error: Error) {
    if let Some(previous) = slot.replace(error) {
        discard_error(previous);
    }
}

pub(super) fn discard_failure(slot: &mut Option<Error>) {
    if let Some(error) = slot.take() {
        discard_error(error);
    }
}

pub(super) fn report_failure(error: Error, message: &'static str) {
    let detail = consume_failure_detail(error);
    let _ = contain(|| tracing::error!(%detail, message));
}

pub(super) fn consume_failure_detail(error: Error) -> String {
    consume_error_detail(error, ERROR_DETAIL_FALLBACK)
}

pub(super) fn consume_error_detail(error: Error, fallback: &'static str) -> String {
    let detail = error_detail(&error, fallback);
    discard_error(error);
    detail
}

pub(super) fn dispose_payload(payload: Box<dyn Any + Send>) {
    if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        std::mem::forget(secondary);
    }
}

fn error_detail(error: &Error, fallback: &'static str) -> String {
    contain(|| format!("{error:#}")).unwrap_or_else(|()| fallback.to_owned())
}

fn discard_error(error: Error) {
    let _ = contain(|| drop(error));
}

fn contain<Value>(operation: impl FnOnce() -> Value) -> Result<Value, ()> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            dispose_payload(payload);
            Err(())
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/captured/tree/force/captured_tree_force_reopen_seal_terminal_tests.rs"]
mod tests;
