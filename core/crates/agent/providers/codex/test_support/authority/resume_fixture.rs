use super::*;
use std::path::{Path, PathBuf};

pub(super) const TEST_RESUME_BOOT: &str = "codex-authority-test-boot";
const ROLLOUT_DIRECTORY: &str = "sessions/2026/10/07";
const ROLLOUT_STAMP: &str = "2026-10-07T12-00-00";

pub(super) fn evidence_path(root: &Path) -> PathBuf {
    root.canonicalize()
        .unwrap()
        .join(ROLLOUT_DIRECTORY)
        .join(format!(
            "{}{ROLLOUT_STAMP}-{SESSION_ID}.{}",
            crate::codex::constants::ROLLOUT_FILE_PREFIX,
            crate::agent::constants::JSONL_EXTENSION,
        ))
}

pub(super) fn write_evidence(root: &Path) {
    let path = evidence_path(root);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let record = serde_json::json!({
        (crate::codex::constants::RECORD_TYPE_KEY): crate::codex::constants::SESSION_META_TYPE,
        (crate::codex::constants::PAYLOAD_KEY): {
            (crate::codex::constants::SESSION_ID_KEY): SESSION_ID,
            (crate::codex::constants::CWD_KEY): root.canonicalize().unwrap(),
            (crate::codex::constants::SOURCE_KEY): crate::codex::constants::CLI_SOURCE,
        }
    });
    std::fs::write(path, format!("{record}\n")).unwrap();
}

impl Fixture {
    pub(super) fn verified_authority(&self, generation: u64) -> CodexBindingAuthority {
        self.authority(generation, CodexAuthorityStatus::Active)
            .with_verified_resume_identity(TEST_RESUME_BOOT)
            .unwrap()
    }
}
