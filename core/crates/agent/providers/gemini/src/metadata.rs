use super::constants;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const HEADER_READ_LIMIT: u64 = 256 * 1024;
const MAX_RESUME_CANDIDATES: usize = 4096;
const TRANSCRIPT_SHORT_ID_BYTES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TranscriptMetadata {
    pub session_id: String,
    pub cwd: PathBuf,
}

impl TranscriptMetadata {
    /// The header carries the session identity and project hash; the agent
    /// process cwd is the fence because Gemini records no cwd in the header.
    pub(super) fn from_read_window(head: &[Value], agent_cwd: &Path) -> Option<Self> {
        let header = head
            .iter()
            .find(|value| value.get("sessionId").is_some() && value.get("projectHash").is_some())?;
        let session_id = normalized_session_id(header.get("sessionId")?.as_str()?)?;
        let cwd = canonical_existing_cwd(agent_cwd)?;
        Some(Self { session_id, cwd })
    }
}

/// Gemini writes one header line and then append-only records. Some records are
/// `$set` patches of header fields, so only the first JSON line is the header.
pub(super) fn transcript_session_id(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = BufReader::new(file.take(HEADER_READ_LIMIT));
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let value: Value = serde_json::from_str(line.trim()).ok()?;
    let session_id = value.get("sessionId")?.as_str()?;
    normalized_session_id(session_id)
}

/// A resumed Gemini session first opens a fresh transcript file and reports it
/// to `SessionStart`, then deletes it and appends to the original session file.
/// The lifecycle events that follow all carry the original path, so the resume
/// start must resolve to the durable file: the largest transcript carrying this
/// session identity in the same chats directory.
pub fn resolve_transcript(path: &Path, session_id: &str) -> Option<PathBuf> {
    let normalized = normalized_session_id(session_id)?;
    let short = normalized.get(..TRANSCRIPT_SHORT_ID_BYTES)?;
    let suffix = format!("-{short}");
    let directory = path.parent()?;
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(directory).ok()?.flatten().take(MAX_RESUME_CANDIDATES) {
        let candidate = entry.path();
        let stem_matches = candidate
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.ends_with(&suffix));
        if !stem_matches
            || !main_transcript_path(&candidate)
            || transcript_session_id(&candidate).as_deref() != Some(normalized.as_str())
        {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        candidates.push((metadata.len(), metadata.modified().ok(), candidate));
    }
    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    candidates
        .pop()
        .map(|(_, _, path)| path)
        .or_else(|| path.is_file().then(|| path.to_path_buf()))
}

pub(super) fn main_transcript_path(path: &Path) -> bool {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
    path.extension().and_then(|extension| extension.to_str())
        == Some(constants::TRANSCRIPT_EXTENSION)
        && path.parent().and_then(Path::file_name).and_then(|name| name.to_str())
            == Some(constants::TRANSCRIPT_DIR)
        && name.starts_with(constants::TRANSCRIPT_PREFIX)
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

pub(super) fn data_home_from_environment(
    environment: &std::collections::BTreeMap<String, String>,
) -> Option<PathBuf> {
    if let Some(home) = environment.get(constants::DATA_HOME_ENV) {
        return Some(Path::new(home).join(constants::DATA_HOME_DIR));
    }
    environment
        .get(constants::HOME_ENV)
        .map(|home| Path::new(home).join(constants::DATA_HOME_DIR))
}

pub(super) fn resume_environment(data_home: &str) -> Option<(String, String)> {
    let parent = Path::new(data_home).parent()?;
    Some((
        constants::DATA_HOME_ENV.to_owned(),
        parent.to_string_lossy().into_owned(),
    ))
}
