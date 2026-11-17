use super::*;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;
use yolo_runtime::agent::constants::{COMMAND_LINE_ENDING, PATH_ENV, SHELL_ENV};
use yolo_runtime::agent::{AgentState, ProcessSource, SystemProcessSource};
use yolo_runtime::{RecoveryState, TerminalView};

const AGENT_BIN_DIR: &str = "agent-bin";
const AGENT_WORKING_DIRECTORY: &str = "agent-workdirs";
const CODEX_DATA_HOME_DIRECTORY: &str = "codex-data";
const CLAUDE_DATA_HOME_DIRECTORY: &str = "claude-data";
const SHELL_SINGLE_QUOTE_ESCAPE: &str = "'\"'\"'";
const TEST_EXECUTABLE_MODE: u32 = 0o755;
const TEST_SHELL: &str = "bash";
const TEST_SHELL_CONTENTS: &str = "#!/bin/sh\nexec /bin/bash --noprofile --norc -i\n";
const PROVIDER_MANAGEMENT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(
    if yolo_runtime::codex_hooks::CODEX_MANAGEMENT_HANDSHAKE_TIMEOUT.as_secs()
        > yolo_runtime::claude_hooks::CLAUDE_MANAGEMENT_HANDSHAKE_TIMEOUT.as_secs()
    {
        yolo_runtime::codex_hooks::CODEX_MANAGEMENT_HANDSHAKE_TIMEOUT.as_secs()
    } else {
        yolo_runtime::claude_hooks::CLAUDE_MANAGEMENT_HANDSHAKE_TIMEOUT.as_secs()
    },
);
const AGENT_STATE_SETTLEMENT_TIMEOUT: Duration =
    yolo_session_timing::SESSION_STARTUP_CLIENT_TIMEOUT
        .saturating_add(PROVIDER_MANAGEMENT_HANDSHAKE_TIMEOUT);
const INVOCATION_QUIET_PERIOD: Duration = yolo_runtime::agent::constants::DETECTION_INTERVAL;
const BINDING_WRITE_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
#[cfg(target_os = "macos")]
const PREVIOUS_BOOT_ID: &str = "macos:550e8400-e29b-41d4-a716-446655440000";
#[cfg(not(target_os = "macos"))]
const PREVIOUS_BOOT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

#[derive(Clone, Copy)]
struct ProviderFixture {
    executable: &'static str,
    kind: &'static str,
    data_home_environment: &'static str,
    data_home_directory: &'static str,
    evidence_directory: &'static str,
    resume_argument: &'static str,
    resume_cwd_argument: Option<&'static str>,
    binary_runfile: &'static str,
}

impl ProviderFixture {
    fn process_executable(self) -> &'static str {
        self.executable
    }

    fn evidence_path(self, data_home: &Path, session_id: &str) -> PathBuf {
        let directory = data_home.join(self.evidence_directory);
        if self.kind == yolo_runtime::agent::codex::constants::KIND {
            return directory
                .join(fake_agent_constants::CODEX_ROLLOUT_DATE_DIRECTORY)
                .join(format!(
                    "{}{}{session_id}",
                    yolo_runtime::agent::codex::constants::ROLLOUT_FILE_PREFIX,
                    fake_agent_constants::CODEX_ROLLOUT_TEST_LABEL,
                ))
                .with_extension(yolo_runtime::agent::constants::JSONL_EXTENSION);
        }
        directory
            .join(fake_agent_constants::PACKAGED_PROJECT_DIRECTORY)
            .join(session_id)
            .with_extension(yolo_runtime::agent::constants::JSONL_EXTENSION)
    }
}

const CODEX_PROVIDER: ProviderFixture = ProviderFixture {
    executable: yolo_runtime::agent::codex::constants::EXECUTABLE,
    kind: yolo_runtime::agent::codex::constants::KIND,
    data_home_environment: yolo_runtime::agent::codex::constants::DATA_HOME_ENV,
    data_home_directory: CODEX_DATA_HOME_DIRECTORY,
    evidence_directory: yolo_runtime::agent::codex::constants::EVIDENCE_DIR,
    resume_argument: yolo_runtime::agent::codex::constants::RESUME_ARGUMENT,
    resume_cwd_argument: Some(yolo_runtime::agent::codex::constants::CWD_ARGUMENT),
    binary_runfile: FAKE_CODEX_BINARY_RUNFILE,
};

const CLAUDE_PROVIDER: ProviderFixture = ProviderFixture {
    executable: yolo_runtime::agent::claude::constants::EXECUTABLE,
    kind: yolo_runtime::agent::claude::constants::KIND,
    data_home_environment: yolo_runtime::agent::claude::constants::DATA_HOME_ENV,
    data_home_directory: CLAUDE_DATA_HOME_DIRECTORY,
    evidence_directory: yolo_runtime::agent::claude::constants::EVIDENCE_DIR,
    resume_argument: yolo_runtime::agent::claude::constants::RESUME_ARGUMENT,
    resume_cwd_argument: None,
    binary_runfile: FAKE_CLAUDE_BINARY_RUNFILE,
};

#[path = "agent_recovery/federation_fixture.rs"]
mod federation_fixture;
#[path = "agent_recovery/matrix.rs"]
mod matrix;
pub(crate) use federation_fixture::{FederationHost, federation_agent_command, federation_harness};

#[path = "agent_recovery/native_hook_registration_race.rs"]
mod native_hook_registration_race;

#[path = "agent_recovery/invocation_log.rs"]
mod invocation_log;
use invocation_log::*;

#[path = "agent_recovery/runtime_snapshot.rs"]
mod runtime_snapshot;
use runtime_snapshot::*;

#[path = "agent_recovery/view_wait.rs"]
mod view_wait;
use view_wait::*;

#[path = "agent_recovery/support.rs"]
mod support;
use support::*;

#[path = "agent_recovery/state_persistence.rs"]
mod state_persistence;
use state_persistence::*;

#[path = "agent_recovery/native_fixture_assertions.rs"]
mod native_fixture_assertions;

#[path = "../fake_agent/invocation.rs"]
mod fake_agent_invocation;
