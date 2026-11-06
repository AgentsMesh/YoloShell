use super::super::LinuxContainmentRecord;
use std::fs;
use std::path::PathBuf;

pub struct DurableRecord {
    _directory: tempfile::TempDir,
    path: PathBuf,
    bytes: Vec<u8>,
}

impl DurableRecord {
    pub fn write(record: &LinuxContainmentRecord) -> Self {
        let directory = tempfile::tempdir().expect("create record directory");
        let path = directory.path().join("containment.json");
        let bytes = serde_json::to_vec(record).expect("serialize containment record");
        fs::write(&path, &bytes).expect("persist containment record");
        Self {
            _directory: directory,
            path,
            bytes,
        }
    }

    pub fn assert_unchanged(&self) {
        assert_eq!(
            fs::read(&self.path).expect("read durable record"),
            self.bytes
        );
    }
}
