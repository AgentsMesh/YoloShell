use super::{CodexAdapter, constants};
use crate::agent::{
    AgentAdapter, AgentDetector, AgentKind, AgentRegistry, DetectionContext, ObservedProcess,
};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{TempDir, tempdir};
use uuid::Uuid;

pub(super) const SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
pub(super) const NEW_SESSION_ID: &str = "7d444840-9dc0-11d1-b245-5ffdce74fad2";
pub(super) const OTHER_SESSION_ID: &str = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
pub(super) const PROCESS_PID: u32 = 41_101;
pub(super) const OTHER_PROCESS_PID: u32 = 41_102;
pub(super) const PROCESS_START_TIME: u64 = 81_101;
pub(super) const OTHER_PROCESS_START_TIME: u64 = 81_102;
const PROCESS_GROUP: i32 = 51_101;
const PROJECT_DIRECTORY: &str = "project";
const DATA_HOME_DIRECTORY: &str = "codex-home";
const ROLLOUT_RELATIVE_DIRECTORY: &str = "sessions/2026/08/05";
const ROLLOUT_STAMP: &str = "2026-08-05T12-00-00";
const JSON_LINE_ENDING: char = '\n';

pub(super) struct Fixture {
    _directory: TempDir,
    pub terminal_id: Uuid,
    pub cwd: PathBuf,
    pub data_home: PathBuf,
    pub rollout_directory: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let directory = tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let cwd = root.join(PROJECT_DIRECTORY);
        let data_home = root.join(DATA_HOME_DIRECTORY);
        let rollout_directory = data_home.join(ROLLOUT_RELATIVE_DIRECTORY);
        fs::create_dir_all(&cwd).unwrap();
        fs::create_dir_all(&rollout_directory).unwrap();
        Self {
            _directory: directory,
            terminal_id: Uuid::new_v4(),
            cwd,
            data_home,
            rollout_directory,
        }
    }

    pub(super) fn evidence(&self, session_id: &str) -> PathBuf {
        self.rollout_directory.join(format!(
            "{}{ROLLOUT_STAMP}-{session_id}.{}",
            constants::ROLLOUT_FILE_PREFIX,
            crate::agent::constants::JSONL_EXTENSION,
        ))
    }

    pub(super) fn metadata(&self, session_id: &str, source: Value) -> Value {
        json!({
            (constants::RECORD_TYPE_KEY): constants::SESSION_META_TYPE,
            (constants::PAYLOAD_KEY): {
                (constants::SESSION_ID_KEY): session_id,
                (constants::CWD_KEY): self.cwd,
                (constants::SOURCE_KEY): source,
            }
        })
    }

    pub(super) fn write_main(&self, session_id: &str) -> PathBuf {
        let event = json!({
            (constants::RECORD_TYPE_KEY): constants::EVENT_MESSAGE_TYPE,
            (constants::PAYLOAD_KEY): {
                (constants::EVENT_KIND_KEY): constants::TASK_COMPLETE_EVENT,
            }
        });
        let path = self.evidence(session_id);
        self.write_values(
            &path,
            &[
                self.metadata(session_id, json!(constants::CLI_SOURCE)),
                event,
            ],
        );
        path
    }

    pub(super) fn write_values(&self, path: &Path, values: &[Value]) {
        let contents = values
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join(&JSON_LINE_ENDING.to_string());
        fs::write(path, format!("{contents}{JSON_LINE_ENDING}")).unwrap();
    }

    pub(super) fn context(&self) -> DetectionContext {
        DetectionContext::new(
            self.terminal_id,
            self.cwd.clone(),
            [(AgentKind::Codex, self.data_home.clone())],
        )
    }

    pub(super) fn inspect(&self, path: &Path) -> Option<crate::agent::AgentEvidence> {
        CodexAdapter
            .inspect_evidence_in_context(path, &self.cwd, &self.context())
            .unwrap()
    }

    pub(super) fn inspect_with_hook(
        &self,
        path: &Path,
        hook: impl FnOnce(),
    ) -> Option<crate::agent::AgentEvidence> {
        CodexAdapter
            .inspect_evidence_with_hook(path, &self.cwd, &self.data_home, hook)
            .unwrap()
    }

    pub(super) fn process(
        &self,
        pid: u32,
        start_time: u64,
        arguments: &[&str],
        associated: Option<&[&str]>,
    ) -> ObservedProcess {
        let mut argv = vec![constants::EXECUTABLE.to_owned()];
        argv.extend(arguments.iter().map(|argument| (*argument).to_owned()));
        ObservedProcess {
            pid,
            start_time,
            parent_pid: crate::agent::constants::MIN_PROCESS_PID,
            process_group: PROCESS_GROUP,
            foreground_group: PROCESS_GROUP,
            executable: constants::EXECUTABLE.to_owned(),
            argv,
            environment: None,
            cwd: Some(self.cwd.clone()),
            evidence_paths: associated.map(|ids| {
                ids.iter()
                    .map(|session_id| self.evidence(session_id))
                    .collect()
            }),
        }
    }

    pub(super) fn detector(&self) -> AgentDetector {
        AgentDetector::new(self.context(), yolo_agent_providers::builtin_agent_registry(), None).unwrap()
    }
}
