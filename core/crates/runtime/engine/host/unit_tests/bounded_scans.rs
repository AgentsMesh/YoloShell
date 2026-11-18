use crate::host::tests::*;
use super::*;

const BOUNDED_SCAN_TEST_LIMIT: usize = 32;
const BOUNDED_SCAN_OVERFLOW_ENTRIES: usize = BOUNDED_SCAN_TEST_LIMIT + 1;
const DURABLE_ENTRY_COUNT: usize = 2;
const FIRST_DURABLE_ENTRY: &str = "first-entry";
const SECOND_DURABLE_ENTRY: &str = "second-entry";

#[test]
fn creation_ledger_parent_counts_every_unrelated_entry() {
    let directory = tempdir().unwrap();
    let paths = Paths::new(directory.path());
    paths.ensure_base().unwrap();
    let ledger = directory.path().join(CREATION_LEDGER_FILE_NAME);
    for index in 0..BOUNDED_SCAN_OVERFLOW_ENTRIES {
        std::fs::write(directory.path().join(format!("unrelated-{index}")), b"keep").unwrap();
    }

    assert!(cleanup_stale_creation_ledgers_bounded(&ledger, BOUNDED_SCAN_TEST_LIMIT).is_err());
    assert_eq!(
        std::fs::read_dir(directory.path()).unwrap().count(),
        BOUNDED_SCAN_OVERFLOW_ENTRIES + 2
    );
}

#[test]
fn durable_session_enumeration_counts_noncanonical_entries() {
    let directory = tempdir().unwrap();
    crate::paths::directory::PrivateDirectory::ensure(directory.path()).unwrap();
    for index in 0..BOUNDED_SCAN_OVERFLOW_ENTRIES {
        std::fs::write(
            directory.path().join(format!("not-a-session-{index}")),
            b"keep",
        )
        .unwrap();
    }
    let durable = catalog_durable_dir::DurableDirectory::open(directory.path()).unwrap();

    assert!(durable.entries_bounded(BOUNDED_SCAN_TEST_LIMIT).is_err());
    assert_eq!(
        std::fs::read_dir(directory.path()).unwrap().count(),
        BOUNDED_SCAN_OVERFLOW_ENTRIES
    );
}

#[test]
fn durable_directory_enumeration_uses_an_independent_cursor() {
    let directory = tempdir().unwrap();
    crate::paths::directory::PrivateDirectory::ensure(directory.path()).unwrap();
    std::fs::write(directory.path().join(FIRST_DURABLE_ENTRY), []).unwrap();
    std::fs::write(directory.path().join(SECOND_DURABLE_ENTRY), []).unwrap();
    let durable = catalog_durable_dir::DurableDirectory::open(directory.path()).unwrap();

    let mut first = durable.entries().unwrap();
    let mut second = durable.entries().unwrap();
    first.sort();
    second.sort();

    assert_eq!(first, second);
    assert_eq!(first.len(), DURABLE_ENTRY_COUNT);
}
