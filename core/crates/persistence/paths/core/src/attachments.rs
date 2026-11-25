use std::path::{Path, PathBuf};

const DIRECTORY: &str = "attachments";

/// Client-supplied files live in the private session directory until archive.
pub fn directory(session: &Path) -> PathBuf {
    session.join(DIRECTORY)
}
