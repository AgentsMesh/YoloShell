use super::*;
use std::os::unix::fs::MetadataExt;

const RUNTIME_BYTES_CHANGED: &str = "fail-closed recovery changed durable runtime bytes";
const RUNTIME_IDENTITY_CHANGED: &str = "fail-closed recovery replaced the durable runtime file";

pub(super) struct DurableRuntimeSnapshot {
    pub(super) record: RuntimeRecord,
    bytes: Vec<u8>,
    device: u64,
    inode: u64,
}

impl DurableRuntimeSnapshot {
    pub(super) fn capture(paths: &Paths, terminal_id: Uuid) -> Self {
        let path = paths.runtime_record(terminal_id);
        let bytes = std::fs::read(&path).unwrap();
        let record = serde_json::from_slice(&bytes).unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        Self {
            record,
            bytes,
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }

    pub(super) fn assert_unchanged(&self, paths: &Paths) {
        let path = paths.runtime_record(self.record.terminal_id);
        let metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(
            (metadata.dev(), metadata.ino()),
            (self.device, self.inode),
            "{RUNTIME_IDENTITY_CHANGED}"
        );
        assert_eq!(
            std::fs::read(path).unwrap(),
            self.bytes,
            "{RUNTIME_BYTES_CHANGED}"
        );
    }
}
