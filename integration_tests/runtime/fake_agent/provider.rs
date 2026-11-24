use crate::fake_agent_constants;
use serde_json::{Value, json};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use yolo_runtime::agent::{claude, codex};

const EXECUTABLE_NAME_MISSING: &str = "fake agent executable name is unavailable";
const PROVIDER_UNSUPPORTED: &str = "fake agent executable does not identify a provider";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FakeAgentProvider {
    Codex,
    Claude,
}

impl FakeAgentProvider {
    pub(crate) fn from_process() -> Result<Self, &'static str> {
        let executable = std::env::args_os().next().ok_or(EXECUTABLE_NAME_MISSING)?;
        Self::from_executable(Path::new(&executable))
    }

    pub(crate) fn from_executable(path: &Path) -> Result<Self, &'static str> {
        match path.file_name() {
            Some(name) if name == OsStr::new(codex::constants::EXECUTABLE) => Ok(Self::Codex),
            Some(name) if name == OsStr::new(claude::constants::EXECUTABLE) => Ok(Self::Claude),
            Some(_) => Err(PROVIDER_UNSUPPORTED),
            None => Err(EXECUTABLE_NAME_MISSING),
        }
    }

    pub(crate) const fn data_home_environment(self) -> &'static str {
        match self {
            Self::Codex => codex::constants::DATA_HOME_ENV,
            Self::Claude => claude::constants::DATA_HOME_ENV,
        }
    }

    pub(crate) const fn evidence_directory(self) -> &'static str {
        match self {
            Self::Codex => codex::constants::EVIDENCE_DIR,
            Self::Claude => claude::constants::EVIDENCE_DIR,
        }
    }

    pub(crate) const fn resume_argument(self) -> &'static str {
        match self {
            Self::Codex => codex::constants::RESUME_ARGUMENT,
            Self::Claude => claude::constants::RESUME_ARGUMENT,
        }
    }

    pub(crate) fn evidence_path(self, directory: &Path, session_id: &str) -> PathBuf {
        match self {
            Self::Codex => directory
                .join(fake_agent_constants::CODEX_ROLLOUT_DATE_DIRECTORY)
                .join(format!(
                    "{}{}{session_id}",
                    codex::constants::ROLLOUT_FILE_PREFIX,
                    fake_agent_constants::CODEX_ROLLOUT_TEST_LABEL,
                ))
                .with_extension(yolo_runtime::agent::constants::JSONL_EXTENSION),
            Self::Claude => directory
                .join(session_id)
                .with_extension(yolo_runtime::agent::constants::JSONL_EXTENSION),
        }
    }

    pub(crate) fn initial_evidence(self, session_id: &str, cwd: &Path) -> Vec<Value> {
        match self {
            Self::Codex => vec![
                json!({
                    (codex::constants::RECORD_TYPE_KEY): codex::constants::SESSION_META_TYPE,
                    (codex::constants::PAYLOAD_KEY): {
                        (codex::constants::SESSION_ID_KEY): session_id,
                        (codex::constants::CWD_KEY): cwd,
                        (codex::constants::SOURCE_KEY): codex::constants::CLI_SOURCE,
                    },
                }),
                json!({
                    (codex::constants::RECORD_TYPE_KEY): codex::constants::EVENT_MESSAGE_TYPE,
                    (codex::constants::PAYLOAD_KEY): {
                        (codex::constants::EVENT_KIND_KEY): codex::constants::TASK_STARTED_EVENT,
                    },
                }),
            ],
            Self::Claude => vec![json!({
                (claude::constants::RECORD_TYPE_KEY): claude::constants::USER_RECORD,
                (claude::constants::SESSION_ID_KEY): session_id,
                (claude::constants::CWD_KEY): cwd,
            })],
        }
    }
}
