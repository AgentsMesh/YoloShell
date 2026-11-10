use crate::host::tests::*;
use super::*;

fn replace_creation_parent(path: &Path) -> PathBuf {
    use std::os::unix::fs::DirBuilderExt;
    let parent = path.parent().unwrap();
    let moved = parent.with_extension("moved");
    std::fs::rename(parent, &moved).unwrap();
    std::fs::DirBuilder::new()
        .mode(crate::host::TEST_PRIVATE_DIRECTORY_MODE)
        .create(parent)
        .unwrap();
    moved
}

#[test]
fn creation_ledger_rejects_parent_replacement_before_publish() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    crate::paths::directory::PrivateDirectory::ensure(&home).unwrap();
    let path = home.join(CREATION_LEDGER_FILE_NAME);
    let mut moved = None;
    let result = write_creation_ledger_with_test_hook(&path, &CreationLedger::default(), |stage| {
        if stage == CreationLedgerWriteStage::ParentOpened {
            moved = Some(replace_creation_parent(&path));
        }
        Ok(())
    });

    assert!(matches!(
        result,
        Err(CreationLedgerWriteFailure::Unchanged(_))
    ));
    assert!(!path.exists());
    assert!(moved.unwrap().is_dir());
}

#[test]
fn creation_ledger_parent_replacement_after_publish_is_uncertain() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    crate::paths::directory::PrivateDirectory::ensure(&home).unwrap();
    let path = home.join(CREATION_LEDGER_FILE_NAME);
    let mut moved = None;
    let result = write_creation_ledger_with_test_hook(&path, &CreationLedger::default(), |stage| {
        if stage == CreationLedgerWriteStage::Published {
            moved = Some(replace_creation_parent(&path));
        }
        Ok(())
    });

    assert!(matches!(
        result,
        Err(CreationLedgerWriteFailure::Uncertain(_))
    ));
    assert!(!path.exists());
    assert!(moved.unwrap().join(CREATION_LEDGER_FILE_NAME).is_file());
}
