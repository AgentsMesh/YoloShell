use super::{STARTUP_CLEANUP_FAILURE_DETAIL, STARTUP_CLEANUP_PANIC_DETAIL};
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(crate) fn contain_drop_cleanup(cleanup: impl FnOnce() -> Result<()>) {
    match catch_unwind(AssertUnwindSafe(cleanup)) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => report_error(error),
        Err(payload) => report_panic(payload),
    }
}

pub(crate) fn report_error(error: anyhow::Error) {
    let detail = super::super::supervision::error_detail(error);
    let _ = super::super::supervision::contain(|| {
        tracing::error!(%detail, STARTUP_CLEANUP_FAILURE_DETAIL);
    });
}

pub(crate) fn report_panic(payload: Box<dyn std::any::Any + Send>) {
    let detail = super::super::supervision::panic_detail(payload);
    let _ = super::super::supervision::contain(|| {
        tracing::error!(%detail, STARTUP_CLEANUP_PANIC_DETAIL);
    });
}
