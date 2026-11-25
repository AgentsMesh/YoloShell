use crate::constants::PERCENT_ENCODED_BYTE_WIDTH;
use std::path::{Path, PathBuf};

/// Derive the session transcript under a provider data home. Grok names the
/// session directory after the percent-encoded working directory and writes
/// the authoritative ACP update stream as `updates.jsonl`.
pub(super) fn session_transcript(home: &Path, cwd: &Path, session_id: &str) -> PathBuf {
    home.join(yolo_grok::grok::constants::EVIDENCE_DIR)
        .join(encode_session_directory(cwd))
        .join(session_id)
        .join(yolo_grok::grok::constants::TRANSCRIPT_FILE)
}

/// Session directories are percent-encoded working directories, keeping the
/// unreserved characters literal (e.g. `/home/x` becomes `%2Fhome%2Fx`).
fn encode_session_directory(cwd: &Path) -> String {
    let bytes = cwd.as_os_str().as_encoded_bytes();
    let mut encoded = String::with_capacity(bytes.len() * PERCENT_ENCODED_BYTE_WIDTH);
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
