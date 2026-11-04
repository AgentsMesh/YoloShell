use std::any::Any;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind as system_catch_unwind};

pub(super) const ERROR_DETAIL_FALLBACK: &str = "broker cleanup child error formatting panicked";

pub(super) fn catch_unwind<Value>(operation: impl FnOnce() -> Value) -> Result<Value, ()> {
    match system_catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            dispose_panic(payload);
            Err(())
        }
    }
}

pub(super) fn discard<Value>(value: Value) {
    let _ = catch_unwind(|| drop(value));
}

pub(super) fn error_detail(error: io::Error) -> String {
    let detail =
        catch_unwind(|| error.to_string()).unwrap_or_else(|()| ERROR_DETAIL_FALLBACK.to_owned());
    discard(error);
    detail
}

pub(super) fn report(error: io::Error, process_id: u32, message: &'static str) {
    let detail = error_detail(error);
    let _ = catch_unwind(|| {
        tracing::warn!(process_id, %detail, message = message);
    });
}

fn dispose_panic(payload: Box<dyn Any + Send>) {
    if let Err(secondary) = system_catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        std::mem::forget(secondary);
    }
}
