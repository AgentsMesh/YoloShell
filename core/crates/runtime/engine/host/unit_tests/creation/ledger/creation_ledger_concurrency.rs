use crate::host::tests::*;
use super::*;

const LEDGER_REWRITE_ITERATIONS: usize = 16;
const LEGACY_LEDGER_RESIDUES: usize = 4;

#[test]
fn concurrent_ledger_writers_never_overwrite_the_locked_target() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    write_creation_ledger(&path, &CreationLedger::default()).unwrap();
    let first = CreationLedger {
        bindings: vec![ledger_test_binding(directory.path())],
        ..CreationLedger::default()
    };
    let second = CreationLedger::default();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();

    std::thread::scope(|scope| {
        let writer_path = path.clone();
        let writer_value = first.clone();
        let writer = scope.spawn(move || {
            write_creation_ledger_with_test_hook(&writer_path, &writer_value, |stage| {
                if stage == CreationLedgerWriteStage::BeforePublish {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                }
                Ok(())
            })
        });
        entered_rx.recv().unwrap();
        let contender =
            write_encoded_creation_ledger(&path, &encode_creation_ledger(&second, &path).unwrap());
        assert!(matches!(
            contender,
            Err(CreationLedgerWriteFailure::Unchanged(_))
        ));
        release_tx.send(()).unwrap();
        assert!(writer.join().unwrap().is_ok());
    });
    assert_eq!(read_creation_ledger(&path).unwrap(), Some(first));
}

#[test]
fn repeated_ledger_rewrites_have_constant_directory_growth() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    write_creation_ledger(&path, &CreationLedger::default()).unwrap();
    write_creation_ledger(&path, &CreationLedger::default()).unwrap();
    let stable_entries = std::fs::read_dir(directory.path()).unwrap().count();
    for _ in 0..LEDGER_REWRITE_ITERATIONS {
        write_creation_ledger(&path, &CreationLedger::default()).unwrap();
        assert_eq!(
            std::fs::read_dir(directory.path()).unwrap().count(),
            stable_entries
        );
    }
}

#[test]
fn legacy_ledger_residues_move_to_bounded_non_destructive_slots() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let mut residues = Vec::new();
    for _ in 0..LEGACY_LEDGER_RESIDUES {
        let residue = directory.path().join(format!(
            "{}{}{CREATION_LEDGER_TEMP_SUFFIX}",
            creation_ledger_temporary_prefix(),
            Uuid::new_v4()
        ));
        std::fs::write(&residue, b"legacy").unwrap();
        set_private_file_mode(&residue);
        residues.push(residue);
    }
    let before = std::fs::read_dir(directory.path()).unwrap().count();
    cleanup_stale_creation_ledgers(&path).unwrap();
    assert!(residues.iter().all(|residue| !residue.exists()));
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), before);
    cleanup_stale_creation_ledgers(&path).unwrap();
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), before);
}
