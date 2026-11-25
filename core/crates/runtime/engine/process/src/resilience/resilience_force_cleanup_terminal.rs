use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

const ERROR_FORMAT_PANIC_DETAIL: &str = "retained process cleanup error formatting panicked";

pub(super) fn report_error(error: anyhow::Error, message: &'static str) {
    let detail = consume_error(error);
    report_detail(&detail, message);
}

pub(super) fn report_message(message: &'static str) {
    let _ = contain(|| tracing::error!(message));
}

pub(super) fn report_detail(detail: &str, message: &'static str) {
    let _ = contain(|| tracing::error!(%detail, message));
}

pub(super) fn contain<Value>(operation: impl FnOnce() -> Value) -> Option<Value> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Some(value),
        Err(payload) => {
            dispose_payload(payload);
            None
        }
    }
}

pub(super) fn dispose_payload(payload: Box<dyn Any + Send>) {
    if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        std::mem::forget(secondary);
    }
}

pub(super) fn consume_error(error: anyhow::Error) -> String {
    let detail =
        contain(|| format!("{error:#}")).unwrap_or_else(|| ERROR_FORMAT_PANIC_DETAIL.to_owned());
    let _ = contain(|| drop(error));
    detail
}
