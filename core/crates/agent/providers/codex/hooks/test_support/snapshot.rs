use crate::agent::hooks::{Reducer, ReducerSnapshot};
use crate::codex_hooks::CodexSnapshotCursor;
use crate::codex_hooks::constants::{MAX_SNAPSHOT_BYTES, SNAPSHOT_FILE_NAME};
use std::path::{Path, PathBuf};

pub(super) const DISPLACED_LOCK_FILE: &str = "displaced-snapshot.lock";
pub(super) const DISPLACED_SNAPSHOT_FILE: &str = "displaced-snapshot.json";
pub(super) const FOREIGN_DIRECTORY: &str = "foreign";
pub(super) const INVOCATION_DIRECTORY: &str = "invocation";
pub(super) const MALFORMED_SNAPSHOT: &[u8] = b"{";
pub(super) const OUTSIDE_FILE: &str = "outside";
pub(super) const RETAINED_DIRECTORY: &str = "retained-invocation";
pub(super) const WRONG_FILE_NAME: &str = "state.json";
pub(super) const SIZE_LIMIT_SENTINEL_BYTES: usize = 1;
pub(super) const OVERSIZED_FILL_BYTE: u8 = b' ';
pub(super) const OVERSIZED_SNAPSHOT_BYTES: usize =
    MAX_SNAPSHOT_BYTES as usize + SIZE_LIMIT_SENTINEL_BYTES;

pub(super) struct SnapshotFixture {
    pub(super) root: tempfile::TempDir,
    pub(super) invocation: PathBuf,
    pub(super) path: PathBuf,
}

impl SnapshotFixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let invocation = root.path().join(INVOCATION_DIRECTORY);
        let path = invocation.join(SNAPSHOT_FILE_NAME);
        Self {
            root,
            invocation,
            path,
        }
    }

    pub(super) fn populated() -> (Self, CodexSnapshotCursor) {
        let fixture = Self::new();
        let (empty, loaded) = CodexSnapshotCursor::open(&fixture.path).unwrap();
        assert!(loaded.is_none());
        let cursor = empty.persist(&snapshot()).unwrap();
        (fixture, cursor)
    }

    pub(super) fn foreign_path(&self) -> PathBuf {
        self.root
            .path()
            .join(FOREIGN_DIRECTORY)
            .join(SNAPSHOT_FILE_NAME)
    }
}

pub(super) fn snapshot() -> ReducerSnapshot {
    let identity = super::identity();
    Reducer::new(
        identity.hook,
        crate::agent::hooks::HookProviderKind::Codex,
        identity.provider_version,
    )
    .unwrap()
    .snapshot()
    .unwrap()
}

pub(super) fn encoded(snapshot: &ReducerSnapshot) -> serde_json::Value {
    serde_json::to_value(snapshot).unwrap()
}

pub(super) fn write_json(path: &Path, value: &impl serde::Serialize) {
    crate::paths::atomic_write_json(path, value).unwrap();
}

pub(super) fn write_bytes(path: &Path, bytes: &[u8]) {
    let entry = crate::paths::private_file::PrivateFileEntry::ensure(path).unwrap();
    let max_bytes = u64::try_from(bytes.len()).unwrap();
    crate::paths::write_private_bytes_guarded(entry, bytes, max_bytes, || Ok(())).unwrap();
}
