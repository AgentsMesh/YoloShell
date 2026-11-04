use std::panic::{AssertUnwindSafe, catch_unwind};

const ERROR_DETAIL_FALLBACK: &str = "maintenance scan failed without reportable detail";

pub(super) fn catch<Value>(operation: impl FnOnce() -> Value) -> Result<Value, ()> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            dispose_panic(payload);
            Err(())
        }
    }
}

pub(super) fn discard<Value>(value: Value) -> bool {
    catch(|| drop(value)).is_ok()
}

pub(super) fn error_detail(error: anyhow::Error) -> String {
    let detail = catch(|| error.to_string()).unwrap_or_else(|()| ERROR_DETAIL_FALLBACK.to_owned());
    discard(error);
    detail
}

pub(super) fn trace(operation: impl FnOnce()) {
    let _ = catch(operation);
}

fn dispose_panic(payload: Box<dyn std::any::Any + Send>) {
    if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        std::mem::forget(secondary);
    }
}
