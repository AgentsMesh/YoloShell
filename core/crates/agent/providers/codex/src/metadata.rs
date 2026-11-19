use super::{constants, evidence_reader};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub(super) struct RolloutMetadata {
    pub session_id: String,
    pub cwd: PathBuf,
}

impl RolloutMetadata {
    pub(super) fn from_head(path: &Path, values: &[Value], agent_cwd: &Path) -> Option<Self> {
        let file_session_id = rollout_session_id(path)?;
        canonical_existing_cwd(agent_cwd)?;
        let mut main = None;
        for value in values.iter().filter(|value| is_metadata(value)) {
            match metadata_source(value)? {
                MetadataSource::Nested => continue,
                MetadataSource::Main => {}
            }
            let current = Self::from_value(file_session_id, value)?;
            if main.as_ref().is_some_and(|previous: &Self| {
                previous.session_id != current.session_id || previous.cwd != current.cwd
            }) {
                return None;
            }
            main = Some(current);
        }
        main
    }

    fn from_value(file_session_id: &str, value: &Value) -> Option<Self> {
        let payload = value.get(constants::PAYLOAD_KEY)?;
        let session_id = payload.get(constants::SESSION_ID_KEY)?.as_str()?;
        if canonical_session_id(session_id).as_deref() != Some(file_session_id) {
            return None;
        }
        let cwd = canonical_recorded_cwd(Path::new(payload.get(constants::CWD_KEY)?.as_str()?))?;
        Some(Self {
            session_id: session_id.to_owned(),
            cwd,
        })
    }
}

enum MetadataSource {
    Main,
    Nested,
}

fn metadata_source(value: &Value) -> Option<MetadataSource> {
    let source = value
        .get(constants::PAYLOAD_KEY)?
        .get(constants::SOURCE_KEY)?;
    if matches!(
        source.as_str(),
        Some(constants::CLI_SOURCE | constants::EXEC_SOURCE)
    ) {
        return Some(MetadataSource::Main);
    }
    source
        .get(constants::SUBAGENT_SOURCE_KEY)
        .is_some_and(|value| value.is_object() || value.is_string())
        .then_some(MetadataSource::Nested)
}

fn is_metadata(value: &Value) -> bool {
    value
        .get(constants::RECORD_TYPE_KEY)
        .and_then(Value::as_str)
        == Some(constants::SESSION_META_TYPE)
}

pub(super) fn display_title(data_home: &Path, session_id: &str) -> Option<String> {
    use std::io::SeekFrom;
    let path = data_home.join(constants::SESSION_INDEX_FILE);
    let file_type = std::fs::symlink_metadata(&path).ok()?.file_type();
    if !file_type.is_file() {
        return None;
    }
    let home = data_home.canonicalize().ok()?;
    let path = path.canonicalize().ok()?;
    if !path.starts_with(&home) {
        return None;
    }
    let metadata = std::fs::metadata(&path).ok()?;
    let start = metadata
        .len()
        .saturating_sub(constants::SESSION_INDEX_TAIL_BYTES);
    let mut file = std::fs::File::open(path).ok()?;
    let bytes = evidence_reader::read_at(
        &mut file,
        SeekFrom::Start(start),
        constants::SESSION_INDEX_TAIL_BYTES,
    )
    .ok()?;
    let mut latest = None;
    for value in evidence_reader::parse_bytes(&bytes) {
        if value
            .get(constants::SESSION_INDEX_ID_KEY)
            .and_then(Value::as_str)
            != Some(session_id)
        {
            continue;
        }
        let Some(title) = value
            .get(constants::SESSION_INDEX_TITLE_KEY)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|title| !title.is_empty())
        else {
            continue;
        };
        if crate::adapter::validation::valid_display_title(title) {
            latest = Some(title.to_owned());
        }
    }
    latest
}

pub(super) fn main_rollout_path(path: &Path) -> bool {
    rollout_session_id(path).is_some()
}

pub(super) fn rollout_session_id(path: &Path) -> Option<&str> {
    if path.extension().and_then(|extension| extension.to_str())
        != Some(crate::agent::constants::JSONL_EXTENSION)
    {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    let session_start = stem.len().checked_sub(constants::UUID_HYPHENATED_BYTES)?;
    let prefix = stem.get(..session_start)?;
    let session_id = stem.get(session_start..)?;
    if prefix.len() <= constants::ROLLOUT_FILE_PREFIX.len()
        || !prefix.starts_with(constants::ROLLOUT_FILE_PREFIX)
        || !prefix.ends_with(constants::ROLLOUT_ID_SEPARATOR)
    {
        return None;
    }
    (canonical_session_id(session_id).as_deref() == Some(session_id)).then_some(session_id)
}

pub(super) fn canonical_session_id(value: &str) -> Option<String> {
    let canonical = normalized_session_id(value)?;
    (canonical == value).then_some(canonical)
}

pub(super) fn normalized_session_id(value: &str) -> Option<String> {
    Some(Uuid::parse_str(value).ok()?.hyphenated().to_string())
}

fn canonical_recorded_cwd(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() || path.components().any(|part| part == Component::ParentDir) {
        return None;
    }
    canonical_existing_cwd(path)
}

pub(super) fn canonical_existing_cwd(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let canonical = path.canonicalize().ok()?;
    canonical.is_dir().then_some(canonical)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/metadata_real_schema_tests.rs"]
mod real_schema_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/metadata_tests.rs"]
mod tests;
