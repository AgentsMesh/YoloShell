use std::any::Any;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(super) fn catch_terminal<Value>(operation: impl FnOnce() -> Value) -> Result<Value, ()> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            dispose_payload(payload);
            Err(())
        }
    }
}

pub(super) fn report_io_error(error: io::Error, process_id: u32, message: &'static str) {
    let detail = catch_terminal(|| error.to_string()).ok();
    let _ = catch_terminal(|| drop(error));
    let _ = catch_terminal(|| match detail {
        Some(detail) => tracing::warn!(process_id, %detail, message),
        None => tracing::warn!(process_id, message),
    });
}

pub(super) fn report_error(error: anyhow::Error, message: &'static str) {
    let detail = catch_terminal(|| error.to_string()).ok();
    let _ = catch_terminal(|| drop(error));
    let _ = catch_terminal(|| match detail {
        Some(detail) => tracing::warn!(%detail, message),
        None => tracing::warn!(message),
    });
}

fn dispose_payload(payload: Box<dyn Any + Send>) {
    match catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        Ok(()) => {}
        Err(secondary) => std::mem::forget(secondary),
    }
}
