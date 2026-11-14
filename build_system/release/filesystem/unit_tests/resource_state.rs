use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

const DESCRIPTION: &str = "indeterminate container custody";
const POISON_FAILURE: &str = "panic while updating resource ownership";

#[test]
fn poisoned_owner_rejects_retirement_and_retain_even_after_guard_drop() {
    let resources = DirectoryResources::default();
    let guard = resources.retain(DESCRIPTION).unwrap();
    let failure = catch_unwind(AssertUnwindSafe(|| {
        let _state = resources.0.lock().unwrap();
        panic!("{POISON_FAILURE}");
    }));
    assert!(failure.is_err());
    drop(guard);
    assert!(resources.0.is_poisoned());
    assert_eq!(
        resources.retain(DESCRIPTION).unwrap_err().to_string(),
        TRACKER_POISONED
    );
    assert_eq!(
        resources.retire().unwrap_err().to_string(),
        TRACKER_POISONED
    );
}
