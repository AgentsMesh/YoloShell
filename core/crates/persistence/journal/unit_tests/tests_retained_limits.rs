use super::tests_retained_hooks::{clear_staging_hook, set_staging_hook};
use super::tests_retained_support::RetainedFixture;
use super::tests_security_support::{REPLACEMENT_BYTES, write_private};
use super::*;
use std::cell::Cell;
use std::fs;
use std::rc::Rc;

const RETAINED_LIMIT_FAULT: &str = "injected retained staging limit failure";
const RETAINED_SENTINEL_PREFIX: &str = "retained-limit-sentinel-";
const REUSABLE_SLOT_ENTRY_COUNT: usize = 1;
const REPEATED_FAILURE_ATTEMPTS: usize = 8;
const FIRST_SENTINEL_INDEX: usize = 0;

#[test]
fn repeated_staging_failures_reuse_one_bounded_slot() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let baseline = fixture.entry_count();
    for _ in 0..REPEATED_FAILURE_ATTEMPTS {
        set_staging_hook(|| Err(std::io::Error::other(RETAINED_LIMIT_FAULT).into()));
        assert!(fixture.reconcile().is_err());
        assert_eq!(fixture.entry_count(), baseline + REUSABLE_SLOT_ENTRY_COUNT);
    }
    clear_staging_hook();

    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn a_full_directory_rejects_a_new_slot_without_deleting_entries() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let parent = fixture.moved_journal.parent().unwrap().to_path_buf();
    let mut index = 0_usize;
    while fixture.entry_count() < crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES {
        let sentinel = parent.join(format!("{RETAINED_SENTINEL_PREFIX}{index}"));
        write_private(&sentinel, REPLACEMENT_BYTES);
        index = index.checked_add(REUSABLE_SLOT_ENTRY_COUNT).unwrap();
    }
    let invoked = Rc::new(Cell::new(false));
    let observed = Rc::clone(&invoked);
    set_staging_hook(move || {
        observed.set(true);
        Err(std::io::Error::other(RETAINED_LIMIT_FAULT).into())
    });

    assert!(fixture.reconcile().is_err());
    clear_staging_hook();

    assert!(!invoked.get());
    assert_eq!(
        fixture.entry_count(),
        crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES
    );
    let first = parent.join(format!("{RETAINED_SENTINEL_PREFIX}{FIRST_SENTINEL_INDEX}"));
    assert_eq!(fs::read(first).unwrap(), REPLACEMENT_BYTES);
    fixture.assert_replacements_unchanged();
}
