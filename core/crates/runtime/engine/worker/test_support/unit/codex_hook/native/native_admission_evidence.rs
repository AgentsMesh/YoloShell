use super::{CodexHookPolicy, CodexNativeRequest, ObservedProcess};
use crate::agent::codex::constants;
use crate::codex_hooks::{CodexHookInput, NATIVE_SCHEMA_VERSION};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub(super) const SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
pub(super) const FOREIGN_SESSION_ID: &str = "7d444840-9dc0-11d1-b245-5ffdce74fad2";
pub(super) const MODEL: &str = "gpt-5";
pub(super) const PERMISSION: &str = "default";
pub(super) const SOURCE: &str = "startup";
pub(super) const SECOND_STAMP: &str = "2026-09-10T02-00-00";
const STAMP: &str = "2026-09-10T01-00-00";
const PROJECT_DIRECTORY: &str = "project";
const DATA_HOME_DIRECTORY: &str = "codex-home";
const ROLLOUT_DIRECTORY: &str = "sessions/2026/09/10";
const AGENT_PID: u32 = 41_501;
const AGENT_START_TIME: u64 = 51_501;
const SHELL_PID: u32 = 41_500;
const FOREGROUND_GROUP: i32 = 61_501;
const LINE_ENDING: char = '\n';
const AGENT_FILE_BYTES: &[u8] = b"native-evidence-fixture";

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) agent: ObservedProcess,
    pub(super) policy: CodexHookPolicy,
    pub(super) request: CodexNativeRequest,
    pub(super) path: PathBuf,
    pub(super) data_home: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().canonicalize().unwrap();
        let cwd = directory.join(PROJECT_DIRECTORY);
        let data_home = directory.join(DATA_HOME_DIRECTORY);
        std::fs::create_dir_all(data_home.join(ROLLOUT_DIRECTORY)).unwrap();
        std::fs::create_dir(&cwd).unwrap();
        let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
        let agent_executable = directory.join(constants::EXECUTABLE);
        std::fs::write(&agent_executable, AGENT_FILE_BYTES).unwrap();
        let session = uuid::Uuid::new_v4();
        let epoch = uuid::Uuid::new_v4();
        let policy = CodexHookPolicy::new(session, epoch, executable.clone()).unwrap();
        let request = CodexNativeRequest {
            schema_version: NATIVE_SCHEMA_VERSION,
            terminal_id: session,
            runtime_epoch: epoch,
            event_id: uuid::Uuid::new_v4(),
            hook_executable: executable.clone(),
            data_home_candidates: vec![data_home.clone()],
            input: CodexHookInput::SessionStart {
                session_id: SESSION_ID.to_owned(),
                transcript_path: None,
                cwd: cwd.clone(),
                model: MODEL.to_owned(),
                permission_mode: PERMISSION.to_owned(),
                source: SOURCE.to_owned(),
            },
        };
        let agent = ObservedProcess {
            pid: AGENT_PID,
            start_time: AGENT_START_TIME,
            parent_pid: SHELL_PID,
            executable: agent_executable.to_string_lossy().into_owned(),
            argv: vec![constants::EXECUTABLE.to_owned()],
            cwd: Some(cwd),
            process_group: FOREGROUND_GROUP,
            foreground_group: FOREGROUND_GROUP,
            environment: Some(BTreeMap::from([
                (
                    crate::codex_hooks::TERMINAL_ENV.to_owned(),
                    session.to_string(),
                ),
                (
                    crate::codex_hooks::RUNTIME_ENV.to_owned(),
                    epoch.to_string(),
                ),
                (
                    constants::DATA_HOME_ENV.to_owned(),
                    data_home.to_string_lossy().into_owned(),
                ),
            ])),
            evidence_paths: None,
        };
        let mut fixture = Self {
            _root: root,
            agent,
            policy,
            request,
            path: PathBuf::new(),
            data_home,
        };
        fixture.path = fixture.write_rollout(SESSION_ID, STAMP);
        fixture.agent.evidence_paths = Some(vec![fixture.path.clone()]);
        fixture
    }

    pub(super) fn rollout_path(&self, session: &str, stamp: &str) -> PathBuf {
        self.data_home.join(ROLLOUT_DIRECTORY).join(format!(
            "{}{stamp}-{session}.{}",
            constants::ROLLOUT_FILE_PREFIX,
            crate::agent::constants::JSONL_EXTENSION,
        ))
    }

    pub(super) fn write_rollout(&self, session: &str, stamp: &str) -> PathBuf {
        self.write_rollout_with_source(session, stamp, constants::CLI_SOURCE)
    }

    pub(super) fn write_rollout_with_source(
        &self,
        session: &str,
        stamp: &str,
        source: &str,
    ) -> PathBuf {
        let path = self.rollout_path(session, stamp);
        let metadata = serde_json::json!({
            (constants::RECORD_TYPE_KEY): constants::SESSION_META_TYPE,
            (constants::PAYLOAD_KEY): {
                (constants::SESSION_ID_KEY): session,
                (constants::CWD_KEY): self.request.cwd(),
                (constants::SOURCE_KEY): source,
            },
        });
        std::fs::write(&path, format!("{metadata}{LINE_ENDING}")).unwrap();
        path
    }

    /// A rollout file created by Codex before it commits `session_meta`.
    pub(super) fn write_pending_rollout(&self, session: &str, stamp: &str) -> PathBuf {
        let path = self.rollout_path(session, stamp);
        std::fs::write(&path, "").unwrap();
        path
    }

    pub(super) fn explicit_path(&mut self, path: PathBuf) {
        let CodexHookInput::SessionStart {
            transcript_path, ..
        } = &mut self.request.input
        else {
            unreachable!()
        };
        *transcript_path = Some(path);
    }
}
