use crate::host::tests::*;
use super::*;

fn active_ledger_staging(home: &Path) -> PathBuf {
    let prefix = creation_ledger_temporary_prefix();
    let entries = std::fs::read_dir(home)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with(&prefix) && name.ends_with(CREATION_LEDGER_TEMP_SUFFIX)
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    entries[0].clone()
}

const BEFORE_PUBLISH_TEST_FAILURE: &str = "injected pre-publish failure";

#[test]
fn ledger_publish_source_swap_rolls_back_without_overwriting_an_inode() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let original = CreationLedger::default();
    write_creation_ledger(&path, &original).unwrap();
    let replacement = b"boundary source replacement";
    let saved = directory.path().join("trusted-ledger-staging");
    let candidate = CreationLedger {
        bindings: vec![ledger_test_binding(directory.path())],
        ..CreationLedger::default()
    };
    let expected = encode_creation_ledger(&candidate, &path).unwrap();

    let result = write_creation_ledger_with_test_hook(&path, &candidate, |stage| {
        if stage == CreationLedgerWriteStage::BeforePublish {
            let staging = active_ledger_staging(directory.path());
            std::fs::rename(&staging, &saved)?;
            std::fs::write(&staging, replacement)?;
            set_private_file_mode(&staging);
        }
        Ok(())
    });

    assert!(matches!(
        result,
        Err(CreationLedgerWriteFailure::Uncertain(_))
    ));
    assert_eq!(read_creation_ledger(&path).unwrap(), Some(original));
    assert_eq!(
        std::fs::read(active_ledger_staging(directory.path())).unwrap(),
        replacement
    );
    assert_eq!(std::fs::read(saved).unwrap(), expected);
}

#[test]
fn ledger_publish_target_swap_preserves_the_replacement_and_old_target() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let original = CreationLedger::default();
    write_creation_ledger(&path, &original).unwrap();
    let original_bytes = std::fs::read(&path).unwrap();
    let replacement = b"boundary target replacement";
    let saved = directory.path().join("saved-ledger-target");
    let candidate = CreationLedger {
        bindings: vec![ledger_test_binding(directory.path())],
        ..CreationLedger::default()
    };

    let result = write_creation_ledger_with_test_hook(&path, &candidate, |stage| {
        if stage == CreationLedgerWriteStage::BeforePublish {
            std::fs::rename(&path, &saved)?;
            std::fs::write(&path, replacement)?;
            set_private_file_mode(&path);
        }
        Ok(())
    });

    assert!(matches!(
        result,
        Err(CreationLedgerWriteFailure::Uncertain(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), replacement);
    assert_eq!(std::fs::read(saved).unwrap(), original_bytes);
}

#[test]
fn ledger_publish_boundary_failure_is_conservatively_uncertain() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let original = CreationLedger::default();
    write_creation_ledger(&path, &original).unwrap();
    let candidate = CreationLedger {
        bindings: vec![ledger_test_binding(directory.path())],
        ..CreationLedger::default()
    };

    let result = write_creation_ledger_with_test_hook(&path, &candidate, |stage| {
        anyhow::ensure!(
            stage != CreationLedgerWriteStage::BeforePublish,
            BEFORE_PUBLISH_TEST_FAILURE
        );
        Ok(())
    });

    assert!(matches!(
        result,
        Err(CreationLedgerWriteFailure::Uncertain(_))
    ));
    assert_eq!(read_creation_ledger(&path).unwrap(), Some(original));
}

#[test]
fn creation_deadline_at_ledger_commit_does_not_poison_or_mutate() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let original = CreationLedger::default();
    write_creation_ledger(&path, &original).unwrap();
    let mut ledger = CreationLedger::open(&path).unwrap();
    let candidate = CreationLedger {
        bindings: vec![ledger_test_binding(directory.path())],
        ..CreationLedger::default()
    };
    let validation_boundary_reached = std::cell::Cell::new(false);

    let error = ledger
        .install_with(
            &path,
            candidate,
            Some(creation_commit_deadline()),
            |stage| match stage {
                CreationLedgerWriteStage::BeforePublish => {
                    validation_boundary_reached.set(true);
                    Ok(())
                }
                _ => Ok(()),
            },
        )
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert!(validation_boundary_reached.get());
    assert!(!ledger.poisoned);
    assert_eq!(read_creation_ledger(&path).unwrap(), Some(original));
}

#[test]
fn ledger_stale_cleanup_source_swap_never_unlinks_the_replacement() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let ledger = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let stale = directory.path().join(format!(
        "{}{}{CREATION_LEDGER_TEMP_SUFFIX}",
        creation_ledger_temporary_prefix(),
        Uuid::new_v4()
    ));
    let saved = directory.path().join("saved-stale-ledger");
    let original = b"trusted stale ledger";
    let replacement = b"stale ledger replacement";
    std::fs::write(&stale, original).unwrap();
    set_private_file_mode(&stale);

    let result = cleanup_stale_creation_ledgers_bounded_with(
        &ledger,
        MAX_CREATION_LEDGER_PARENT_ENTRIES,
        |candidate| {
            std::fs::rename(candidate, &saved)?;
            std::fs::write(candidate, replacement)?;
            set_private_file_mode(candidate);
            Ok(())
        },
    );

    assert!(result.is_err());
    assert_eq!(std::fs::read(stale).unwrap(), replacement);
    assert_eq!(std::fs::read(saved).unwrap(), original);
}
