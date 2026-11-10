use super::*;
use std::path::{Path, PathBuf};

pub(super) const FIRST_SLOT_INDEX: usize = 0;
pub(super) const FIRST_GENERATION: &str = "first-generation";
pub(super) const SECOND_GENERATION: &str = "second-generation";
pub(super) const MOVED: &str = "moved";
pub(super) const ORIGINAL: &[u8] = b"original-generation";
pub(super) const REPLACEMENT: &[u8] = b"replacement-generation";
pub(super) const GATE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub(super) struct Fixture {
    pub base: tempfile::TempDir,
    pub root: GenerationRoot,
    pub slot: PathBuf,
    pub payload: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let base = tempfile::tempdir().unwrap();
        let root = GenerationRoot::ensure(&base.path().join(CONFIG_GENERATIONS_DIRECTORY)).unwrap();
        let slot = root
            .path()
            .join(format!("{PENDING_GENERATION_PREFIX}{FIRST_SLOT_INDEX}"));
        let payload = root.path().join(format!(
            "{PENDING_GENERATION_PREFIX}{FIRST_SLOT_INDEX}{PENDING_GENERATION_PAYLOAD_SUFFIX}"
        ));
        Self {
            base,
            root,
            slot,
            payload,
        }
    }

    pub fn publish(&self, pending: &mut PendingDirectory, target: &str, bytes: &[u8]) {
        pending
            .write_file(&entry_name(GENERATION_CONFIG_FILE).unwrap(), bytes)
            .unwrap();
        pending.seal(&self.root).unwrap();
        assert!(
            pending
                .publish(&self.root, &entry_name(target).unwrap())
                .unwrap()
        );
        self.root.sync_and_verify().unwrap();
    }

    pub fn stale_payload(&self) {
        drop(PendingDirectory::create(&self.root).unwrap());
        write_sealed(&self.payload, ORIGINAL);
    }
}

pub(super) fn write_sealed(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path).unwrap();
    fs::write(path.join(GENERATION_CONFIG_FILE), bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(IMMUTABLE_DIRECTORY_MODE)).unwrap();
}

pub(super) fn assert_sealed(path: &Path, bytes: &[u8]) {
    assert_eq!(fs::read(path.join(GENERATION_CONFIG_FILE)).unwrap(), bytes);
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & PERMISSION_BITS_MASK,
        IMMUTABLE_DIRECTORY_MODE
    );
}
