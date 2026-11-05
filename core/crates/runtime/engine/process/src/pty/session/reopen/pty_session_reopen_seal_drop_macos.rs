use super::{MacosPtyReopenSeal, SEAL_DROP_RESTORE_LOG, SealPhase};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

const SEAL_DROP_RESTORE_PANIC: &str = "PTY reopen-seal restore panicked during Drop";

impl Drop for MacosPtyReopenSeal {
    fn drop(&mut self) {
        if !matches!(self.phase, SealPhase::Prepared | SealPhase::Acquired) {
            return;
        }
        match catch_unwind(AssertUnwindSafe(|| self.restore_before_commit())) {
            Ok(Ok(())) => return,
            Ok(Err(error)) => report_error(error),
            Err(payload) => report_panic(payload),
        }
    }
}

fn report_error(error: anyhow::Error) {
    let detail = catch_terminal(|| error.to_string());
    let _ = catch_terminal(|| drop(error));
    trace(detail.as_deref(), SEAL_DROP_RESTORE_LOG);
}

fn report_panic(payload: Box<dyn Any + Send>) {
    let detail = catch_terminal(|| {
        payload
            .downcast_ref::<&str>()
            .map(|value| (*value).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
    })
    .flatten();
    dispose_payload(payload);
    trace(detail.as_deref(), SEAL_DROP_RESTORE_PANIC);
}

fn trace(detail: Option<&str>, message: &'static str) {
    let _ = catch_terminal(|| match detail {
        Some(detail) => tracing::error!(%detail, message),
        None => tracing::error!(message),
    });
}

fn catch_terminal<Value>(operation: impl FnOnce() -> Value) -> Option<Value> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Some(value),
        Err(payload) => {
            dispose_payload(payload);
            None
        }
    }
}

fn dispose_payload(payload: Box<dyn Any + Send>) {
    if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        std::mem::forget(secondary);
    }
}
