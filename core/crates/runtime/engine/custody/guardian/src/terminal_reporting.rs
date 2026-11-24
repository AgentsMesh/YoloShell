use std::panic::{AssertUnwindSafe, catch_unwind};

const ERROR_DETAIL_FALLBACK: &str = "custody guardian error formatting panicked";

pub(super) fn error_ref(error: &anyhow::Error, message: &'static str) {
    let detail = error_ref_detail(error);
    trace(|| tracing::error!(%detail, message));
}

pub(super) fn error_owned(error: anyhow::Error, message: &'static str) {
    let detail = error_detail(error);
    trace(|| tracing::error!(%detail, message));
}

pub(super) fn error_pair_owned(
    primary: anyhow::Error,
    secondary: anyhow::Error,
    message: &'static str,
) {
    let primary_detail = error_detail(primary);
    let secondary_detail = error_detail(secondary);
    trace(|| tracing::error!(%primary_detail, %secondary_detail, message));
}

pub(super) fn warn_owned(error: anyhow::Error, message: &'static str) {
    let detail = error_detail(error);
    trace(|| tracing::warn!(%detail, message));
}

pub(super) fn error_detail(error: anyhow::Error) -> String {
    let detail = error_ref_detail(&error);
    discard(error);
    detail
}

pub(super) fn discard_owned(error: anyhow::Error) {
    discard(error);
}

fn error_ref_detail(error: &anyhow::Error) -> String {
    contain(|| format!("{error:#}")).unwrap_or_else(|()| ERROR_DETAIL_FALLBACK.to_owned())
}

fn trace(operation: impl FnOnce()) {
    let _ = contain(operation);
}

fn discard<Value>(value: Value) {
    let _ = contain(|| drop(value));
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

fn dispose_payload(payload: Box<dyn std::any::Any + Send>) {
    if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        std::mem::forget(secondary);
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/terminal_reporting_tests.rs"]
mod tests;
