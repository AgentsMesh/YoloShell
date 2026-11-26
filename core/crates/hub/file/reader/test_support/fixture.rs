use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use yolo_hub_file_protocol::*;
use yolo_hub_file_reader::ReadRoot;
use yolo_hub_identity::{FileRootEpoch, FileRootId, HubId};

pub const PAYLOAD: &[u8] = b"hello";
pub const CHANGED: &[u8] = b"modified contents";
pub const FILE_PATH: &str = "src/main.rs";
pub const FILE_NAME: &str = "main.rs";
pub const SOURCE_DIRECTORY: &str = "src";
pub const WORKSPACE_DIRECTORY: &str = "project";
pub const PRIVATE_DIRECTORY: &str = "private";
pub const SAVED_NAME: &str = "retained-original";
pub const LINK_NAME: &str = "symlink";
pub const FIFO_NAME: &str = "fifo";
pub const ROOT_NAME: &str = "Workspace";
pub const READ_LENGTH: u32 = 2;
pub const SINGLE_BYTE: u32 = 1;
pub const PRIVATE_MODE: u32 = 0o600;
pub const PROJECT_MODE: u32 = 0o755;

pub struct Fixture {
    _directory: TempDir,
    pub base: PathBuf,
    pub workspace: PathBuf,
    pub private: PathBuf,
    pub summary: FileRootSummary,
}

impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path().canonicalize().unwrap();
        let workspace = base.join(WORKSPACE_DIRECTORY);
        let private = base.join(PRIVATE_DIRECTORY);
        fs::create_dir_all(workspace.join(SOURCE_DIRECTORY)).unwrap();
        fs::create_dir(&private).unwrap();
        fs::write(workspace.join(FILE_PATH), PAYLOAD).unwrap();
        let summary = FileRootSummary::new(FileRootSummaryFields {
            target: FileRootTarget {
                address: FileRootAddress {
                    hub_id: HubId::new(),
                    root_id: FileRootId::new(),
                },
                epoch: FileRootEpoch::new(),
            },
            name: ROOT_NAME.into(),
            path: workspace.to_str().unwrap().into(),
        })
        .unwrap();
        Self {
            _directory: directory,
            base,
            workspace,
            private,
            summary,
        }
    }
    pub fn root(&self) -> ReadRoot {
        ReadRoot::open(self.summary.clone(), std::slice::from_ref(&self.private)).unwrap()
    }
    pub fn request(&self, path: &str) -> ReadFileRequest {
        ReadFileRequest::new(ReadFileFields {
            schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
            target: self.summary.fields().target,
            relative_path: RelativeFilePath::new(path.into()).unwrap(),
            offset: 0,
            length: READ_LENGTH,
            expected_version: None,
        })
        .unwrap()
    }
}
