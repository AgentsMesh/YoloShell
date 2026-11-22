use super::{constants, evidence_path::AnchoredEvidence};
use std::{fs, io::Read, path::PathBuf};

pub const WORKSPACE: &str = "evidence-anchor-workspace";
pub const DATA_HOME: &str = "data-home";
pub const PROJECT_SCOPE: &str = "project-scope";
pub const TRANSCRIPT: &str = "e3ef2aad-a247-4f12-8222-255e23c46b88.jsonl";
pub const DISPLACED: &str = "displaced";
pub const PRESERVED_BYTES: &[u8] = b"retained transcript bytes\n";

pub struct Fixture {
    _directory: tempfile::TempDir,
    pub workspace: PathBuf,
    pub root: PathBuf,
    pub scope: PathBuf,
    pub transcript: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let workspace = directory.path().canonicalize().unwrap().join(WORKSPACE);
        let root = workspace.join(DATA_HOME);
        let scope = root.join(constants::EVIDENCE_DIR).join(PROJECT_SCOPE);
        fs::create_dir_all(&scope).unwrap();
        let transcript = scope.join(TRANSCRIPT);
        fs::write(&transcript, PRESERVED_BYTES).unwrap();
        Self {
            _directory: directory,
            workspace,
            root,
            scope,
            transcript,
        }
    }

    pub(super) fn open(&self) -> AnchoredEvidence {
        AnchoredEvidence::open(&self.root, &self.transcript)
            .unwrap()
            .unwrap()
    }
}

pub(super) fn assert_retained_bytes(anchor: &mut AnchoredEvidence) {
    let mut bytes = Vec::new();
    anchor.file_mut().read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, PRESERVED_BYTES);
}
