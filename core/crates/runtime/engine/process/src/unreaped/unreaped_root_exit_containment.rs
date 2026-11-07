use super::{CHILD_PID_QUERY_PANIC_DETAIL, CHILD_PID_UNAVAILABLE_DETAIL, OwnedPtyChild};
use anyhow::{Context, Result, anyhow};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(super) fn child_process_id(child: &OwnedPtyChild) -> Result<u32> {
    match catch_unwind(AssertUnwindSafe(|| child.process_id())) {
        Ok(process_id) => process_id.context(CHILD_PID_UNAVAILABLE_DETAIL),
        Err(payload) => {
            dispose_payload(payload);
            Err(anyhow!(CHILD_PID_QUERY_PANIC_DETAIL))
        }
    }
}

fn dispose_payload(payload: Box<dyn std::any::Any + Send>) {
    if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        std::mem::forget(secondary);
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/unreaped/unreaped_root_exit_containment_tests.rs"]
mod tests;
