use super::constants;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Grok's base directory: `$GROK_HOME` when set, otherwise `$HOME/.grok`.
pub(super) fn data_home_from_environment(
    environment: &BTreeMap<String, String>,
) -> Option<PathBuf> {
    environment
        .get(constants::DATA_HOME_ENV)
        .map(PathBuf::from)
        .or_else(|| {
            environment
                .get(constants::HOME_ENV)
                .map(|home| Path::new(home).join(constants::DATA_HOME_DIR))
        })
}

pub(super) fn resume_environment(data_home: &str) -> Option<(String, String)> {
    Some((
        constants::DATA_HOME_ENV.to_owned(),
        data_home.to_owned(),
    ))
}

pub(super) fn normalized_session_id(value: &str) -> Option<String> {
    let parsed = Uuid::parse_str(value).ok()?;
    let canonical = parsed.hyphenated().to_string();
    (canonical == value && canonical.len() == constants::SESSION_ID_BYTES).then_some(canonical)
}

pub(super) fn canonical_existing_cwd(path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    canonical.is_dir().then_some(canonical)
}

/// The session identity is the directory name that holds `updates.jsonl`.
pub(super) fn transcript_session_id(path: &Path) -> Option<String> {
    if path.file_name().and_then(|name| name.to_str()) != Some(constants::TRANSCRIPT_FILE) {
        return None;
    }
    normalized_session_id(path.parent()?.file_name()?.to_str()?)
}

pub(super) fn main_transcript_path(path: &Path) -> bool {
    transcript_session_id(path).is_some()
}
