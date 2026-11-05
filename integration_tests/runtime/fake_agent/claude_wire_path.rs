use std::path::{Path, PathBuf};

pub(crate) fn transcript(data_home: &Path, project: &str, session_id: &str) -> PathBuf {
    data_home
        .join(yolo_runtime::agent::claude::constants::EVIDENCE_DIR)
        .join(project)
        .join(session_id)
        .with_extension(yolo_runtime::agent::constants::JSONL_EXTENSION)
}
