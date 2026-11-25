use super::*;

pub(super) fn receipt_test_setup() -> (
    tempfile::TempDir,
    CreationReceiptStore,
    CreationBinding,
    PathBuf,
) {
    let directory = tempdir().unwrap();
    let paths = Paths::new(directory.path());
    paths.ensure_base().unwrap();
    let store = CreationReceiptStore::open(directory.path()).unwrap();
    let binding = ledger_test_binding(directory.path());
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    (directory, store, binding, path)
}
