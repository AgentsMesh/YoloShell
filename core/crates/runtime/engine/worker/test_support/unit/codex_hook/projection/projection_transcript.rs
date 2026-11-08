use super::{Authority, PROVIDER_SESSION, TRANSCRIPT_FILE};
use crate::agent::{codex::constants as codex, constants::COMMAND_LINE_ENDING};

const TRANSCRIPT_YEAR: &str = "2026";
const TRANSCRIPT_MONTH: &str = "08";
const TRANSCRIPT_DAY: &str = "29";

pub(super) fn write(authority: &Authority) -> std::path::PathBuf {
    let directory = authority
        .request
        .data_home
        .join(codex::EVIDENCE_DIR)
        .join(TRANSCRIPT_YEAR)
        .join(TRANSCRIPT_MONTH)
        .join(TRANSCRIPT_DAY);
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(TRANSCRIPT_FILE);
    let metadata = serde_json::json!({
        (codex::RECORD_TYPE_KEY): codex::SESSION_META_TYPE,
        (codex::PAYLOAD_KEY): {
            (codex::SESSION_ID_KEY): PROVIDER_SESSION,
            (codex::CWD_KEY): authority.request.cwd,
            (codex::SOURCE_KEY): codex::CLI_SOURCE,
        }
    });
    std::fs::write(&path, format!("{metadata}{COMMAND_LINE_ENDING}")).unwrap();
    path
}
