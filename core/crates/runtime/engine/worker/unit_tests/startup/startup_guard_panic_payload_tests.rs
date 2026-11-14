use std::panic::{AssertUnwindSafe, catch_unwind};

const PAYLOAD_DROP_PANIC_DETAIL: &str = "injected startup cleanup payload Drop panic";
const OUTER_PANIC_DETAIL: &str = "injected outer startup unwind";

struct PanicOnDropPayload;
struct CleanupBoundary;

impl Drop for PanicOnDropPayload {
    fn drop(&mut self) {
        panic!("{PAYLOAD_DROP_PANIC_DETAIL}");
    }
}

impl Drop for CleanupBoundary {
    fn drop(&mut self) {
        super::contain_drop_cleanup(|| std::panic::panic_any(PanicOnDropPayload));
    }
}

#[test]
fn startup_cleanup_panic_payload_drop_cannot_escape_the_drop_boundary() {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        super::contain_drop_cleanup(|| std::panic::panic_any(PanicOnDropPayload));
    }));

    assert!(outcome.is_ok());
}

#[test]
fn startup_cleanup_secondary_payload_panic_is_contained_during_outer_unwind() {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _boundary = CleanupBoundary;
        panic!("{OUTER_PANIC_DETAIL}");
    }));

    let payload = outcome.unwrap_err();
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    assert_eq!(detail, Some(OUTER_PANIC_DETAIL));
}
