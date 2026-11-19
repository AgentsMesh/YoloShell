use super::*;

pub(super) fn shard_path(fixture: &Fixture, label: &str) -> PathBuf {
    let catalog = fixture._temp.path().join(label).join("catalog.sqlite");
    let name = crate::coordination::catalog_alias_shard_name(&catalog).unwrap();
    fixture.aliases.path().join(name)
}

pub(super) fn run_pass(scanner: &mut CatalogAliasScanner) {
    for _ in 0..MAX_SCAN_CHUNKS {
        match scanner.scan_chunk().unwrap() {
            ScanChunk::PassComplete => return,
            ScanChunk::Busy => std::thread::yield_now(),
            ScanChunk::Progress => {}
        }
    }
    panic!("catalog alias scanner did not complete a pass");
}

pub(super) fn run_until_error(scanner: &mut CatalogAliasScanner) -> anyhow::Error {
    for _ in 0..MAX_SCAN_CHUNKS {
        if let Err(error) = scanner.scan_chunk() {
            return error;
        }
    }
    panic!("catalog alias scanner did not surface injected I/O failure");
}

pub(super) fn private_directory(path: &Path) {
    std::fs::DirBuilder::new()
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(path)
        .unwrap();
}
