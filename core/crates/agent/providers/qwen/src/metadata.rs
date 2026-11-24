use super::constants;
use serde_json::Value;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TranscriptMetadata {
    pub session_id: String,
    pub cwd: PathBuf,
}

impl TranscriptMetadata {
    pub(super) fn from_read_window(
        path: &Path,
        head: &[Value],
        values: &[Value],
        agent_cwd: &Path,
    ) -> Option<Self> {
        let filename_session = transcript_session_id(path)?;
        let record = head.iter().chain(values).find_map(record_metadata)?;
        if record.session_id != filename_session {
            return None;
        }
        let cwd = canonical_existing_cwd(&record.cwd)?;
        let agent_cwd = canonical_existing_cwd(agent_cwd)?;
        (cwd == agent_cwd).then_some(Self {
            session_id: record.session_id,
            cwd,
        })
    }
}

struct RecordMetadata {
    session_id: String,
    cwd: PathBuf,
}

fn record_metadata(value: &Value) -> Option<RecordMetadata> {
    let session_id = value.get("sessionId")?.as_str()?;
    let session_id = normalized_session_id(session_id)?;
    let cwd = value.get("cwd")?.as_str()?;
    let cwd = PathBuf::from(cwd);
    cwd.is_absolute().then_some(RecordMetadata { session_id, cwd })
}

pub(super) fn main_transcript_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        == Some(constants::TRANSCRIPT_EXTENSION)
        && path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some(constants::TRANSCRIPT_DIR)
        && transcript_session_id(path).is_some()
}

pub(super) fn transcript_session_id(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    normalized_session_id(stem)
}

pub(super) fn normalized_session_id(value: &str) -> Option<String> {
    let parsed = Uuid::parse_str(value).ok()?;
    let canonical = parsed.hyphenated().to_string();
    (canonical == value && canonical.len() == constants::SESSION_ID_BYTES).then_some(canonical)
}

pub(super) fn canonical_existing_cwd(path: &Path) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(path).ok()?;
    canonical.is_dir().then_some(canonical)
}

#[cfg(test)]
#[path = "../unit_tests/metadata_tests.rs"]
mod tests;
