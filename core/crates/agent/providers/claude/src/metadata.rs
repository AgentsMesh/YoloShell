use super::constants;
use crate::agent::files::same_directory;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

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
        let file_session_id = transcript_session_id(path)?;
        safe_cwd(agent_cwd)?;
        let (head_has_complete, _) = scan_cwds(head, file_session_id)?;
        if !head_has_complete {
            return None;
        }
        let (_, latest_cwd) = scan_cwds(values, file_session_id)?;
        if !latest_cwd.is_some_and(|cwd| same_directory(&cwd, agent_cwd)) {
            return None;
        }
        Some(Self {
            session_id: file_session_id.to_owned(),
            cwd: agent_cwd.to_path_buf(),
        })
    }
}

fn scan_cwds(values: &[Value], file_session_id: &str) -> Option<(bool, Option<PathBuf>)> {
    let mut has_complete = false;
    let mut latest_cwd = None;
    for value in values {
        let record_session_id = session_field(value)?;
        let record_cwd = cwd_field(value)?;
        if record_session_id.is_some_and(|session_id| session_id != file_session_id) {
            return None;
        }
        if record_cwd.is_some() && record_session_id.is_none() {
            return None;
        }
        if let (Some(_), Some(cwd)) = (record_session_id, record_cwd) {
            has_complete = true;
            latest_cwd = Some(cwd);
        }
    }
    Some((has_complete, latest_cwd))
}

pub(super) fn display_title(values: &[Value], session_id: &str) -> Option<String> {
    values.iter().rev().find_map(|value| {
        if value.get(constants::RECORD_TYPE_KEY)?.as_str() != Some(constants::AI_TITLE_RECORD)
            || value.get(constants::SESSION_ID_KEY)?.as_str() != Some(session_id)
        {
            return None;
        }
        let title = value.get(constants::AI_TITLE_KEY)?.as_str()?.trim();
        (!title.is_empty() && crate::adapter::validation::valid_display_title(title))
            .then(|| title.to_owned())
    })
}

pub(super) fn main_transcript_path(path: &Path) -> bool {
    transcript_session_id(path).is_some()
}

fn transcript_session_id(path: &Path) -> Option<&str> {
    if path.extension().and_then(|extension| extension.to_str())
        != Some(crate::agent::constants::JSONL_EXTENSION)
    {
        return None;
    }
    let name = path.file_name()?.to_str()?;
    if name.starts_with(constants::SUBAGENT_TRANSCRIPT_PREFIX) {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    let parsed = Uuid::parse_str(stem).ok()?;
    (parsed.hyphenated().to_string() == stem).then_some(stem)
}

fn text_field<'a>(value: &'a Value, key: &str) -> Option<Option<&'a str>> {
    match value.get(key) {
        None => Some(None),
        Some(Value::String(text)) => Some(Some(text)),
        Some(_) => None,
    }
}

fn session_field(value: &Value) -> Option<Option<&str>> {
    let field = text_field(value, constants::SESSION_ID_KEY)?;
    (!field.is_some_and(|text| text.is_empty() || text.trim() != text)).then_some(field)
}

fn cwd_field(value: &Value) -> Option<Option<PathBuf>> {
    match text_field(value, constants::CWD_KEY)? {
        None => Some(None),
        Some(value) => safe_cwd(Path::new(value)).map(Some),
    }
}

fn safe_cwd(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() || path.components().any(|part| part == Component::ParentDir) {
        return None;
    }
    Some(path.to_path_buf())
}
