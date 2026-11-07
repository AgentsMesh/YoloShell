use crate::host::tests::*;
use super::*;

fn assert_creation_ledger_rejected_on_open(path: &Path, ledger: &CreationLedger) {
    write_creation_ledger(path, ledger).unwrap();
    assert!(CreationLedger::open(path).is_err());
}

#[test]
fn creation_ledger_rejects_corruption_noncanonical_digests_and_capacity_overflow() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    std::fs::write(&path, b"{").unwrap();
    set_private_file_mode(&path);
    assert!(CreationLedger::open(&path).is_err());

    let mut ledger = CreationLedger::default();
    let mut binding = ledger_test_binding(directory.path());
    binding.creation_token_digest = binding.creation_token_digest.to_uppercase();
    ledger.bindings.push(binding);
    assert!(ledger.validate().is_err());

    let mut binding = ledger_test_binding(directory.path());
    binding.launch.as_mut().unwrap().auth_token = "A".repeat(WORKER_AUTH_TOKEN_HEX_LENGTH);
    let ledger = CreationLedger {
        bindings: vec![binding],
        ..CreationLedger::default()
    };
    assert!(ledger.validate().is_err());

    let mut binding = ledger_test_binding(directory.path());
    binding.launch.as_mut().unwrap().worker_id = Uuid::nil();
    let ledger = CreationLedger {
        bindings: vec![binding],
        ..CreationLedger::default()
    };
    assert!(ledger.validate().is_err());

    let ledger = CreationLedger {
        bindings: vec![ledger_test_binding(directory.path()); MAX_CREATION_LEDGER_ENTRIES + 1],
        ..CreationLedger::default()
    };
    assert!(ledger.validate().is_err());
}

#[test]
fn creation_ledger_open_rejects_nil_product_identities() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);

    let mut nil_session = ledger_test_binding(directory.path());
    nil_session.result.terminal_id = Uuid::nil();
    assert_creation_ledger_rejected_on_open(
        &path,
        &CreationLedger {
            bindings: vec![nil_session],
            ..CreationLedger::default()
        },
    );

    let mut nil_capability = ledger_test_binding(directory.path());
    nil_capability.result.creation_capability = Uuid::nil();
    assert_creation_ledger_rejected_on_open(
        &path,
        &CreationLedger {
            bindings: vec![nil_capability],
            ..CreationLedger::default()
        },
    );
}

#[test]
fn creation_ledger_open_validates_cwd_lexically_without_resolving_it() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);

    for cwd in [
        PathBuf::from("relative/cwd"),
        PathBuf::from("/tmp/../creation-cwd"),
        PathBuf::from("/tmp/./creation-cwd"),
    ] {
        let mut binding = ledger_test_binding(directory.path());
        binding.spec.cwd = cwd;
        assert_creation_ledger_rejected_on_open(
            &path,
            &CreationLedger {
                bindings: vec![binding],
                ..CreationLedger::default()
            },
        );
    }

    let missing = directory.path().join("deleted-cwd");
    assert!(!missing.exists());
    let mut binding = ledger_test_binding(directory.path());
    binding.spec.cwd = missing;
    write_creation_ledger(
        &path,
        &CreationLedger {
            bindings: vec![binding],
            ..CreationLedger::default()
        },
    )
    .unwrap();
    assert!(CreationLedger::open(&path).is_ok());
}
