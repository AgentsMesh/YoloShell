use super::ingress_test_support::IngressEndpoint;
use super::support::Fixture;
use crate::agent::authority::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::agent::codex::authority::{
    CodexAuthorityStatus, CodexAuthorityVersion, CodexBindingAuthority,
};
use crate::agent::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use crate::codex_hooks::CodexAdmissionAuthority;
use uuid::Uuid;

const AUTHORITY_REVISION: u64 = 2;
const EVENT_SEQUENCE: u64 = 1;
const DETECTED_AT_MS: i64 = 100;
const UPDATED_AT_MS: i64 = 200;
const ROLLOUT_YEAR: &str = "2026";
const ROLLOUT_MONTH: &str = "08";
const ROLLOUT_DAY: &str = "29";
const ROLLOUT_TIMESTAMP: &str = "2026-08-29T00-00-00-";
const JSON_LINE_ENDING: char = '\n';

pub(super) fn confirmed_exit_authority(
    endpoint: &IngressEndpoint,
    fixture: &Fixture,
    admission: &CodexAdmissionAuthority,
) -> CodexBindingAuthority {
    let request = endpoint.admission_context(fixture);
    let provider_session_id = Uuid::new_v4().hyphenated().to_string();
    let evidence_path = write_evidence(&request, &provider_session_id);
    let binding = AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: Fixture::terminal_id(),
        kind: AgentKind::Codex,
        provider_session_id,
        executable: crate::agent::codex::constants::EXECUTABLE.to_owned(),
        cwd: request.cwd.display().to_string(),
        evidence_path: evidence_path.display().to_string(),
        data_home: request.data_home.display().to_string(),
        process_pid: None,
        process_start_time: None,
        state: AgentState::Exited,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: DETECTED_AT_MS,
        updated_at_ms: UPDATED_AT_MS,
    };
    CodexBindingAuthority::new_with_verified_resume_identity(
        successor_runtime(),
        fixture.identity.process,
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        CodexAuthorityVersion {
            route_revision: admission.record().route_revision(),
            invocation_id: fixture.identity.invocation_id,
            generation: fixture.identity.generation,
            authority_revision: AUTHORITY_REVISION,
            sequence: EVENT_SEQUENCE,
        },
        CodexAuthorityStatus::Active,
        binding,
        &crate::process::boot_id(),
    )
    .unwrap()
}

pub(super) fn successor_runtime() -> AuthorityRuntimeIdentity {
    let worker = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    AuthorityRuntimeIdentity {
        runtime_epoch: Uuid::new_v4(),
        worker: AuthorityProcessIdentity {
            pid: worker.pid,
            start_time: worker.start_time,
        },
    }
}

pub(super) fn write_evidence(
    request: &crate::codex_hooks::CodexAdmissionContext,
    provider_session_id: &str,
) -> std::path::PathBuf {
    let directory = request
        .data_home
        .join(crate::agent::codex::constants::EVIDENCE_DIR)
        .join(ROLLOUT_YEAR)
        .join(ROLLOUT_MONTH)
        .join(ROLLOUT_DAY);
    std::fs::create_dir_all(&directory).unwrap();
    let filename = format!(
        "{}{ROLLOUT_TIMESTAMP}{provider_session_id}.jsonl",
        crate::agent::codex::constants::ROLLOUT_FILE_PREFIX
    );
    let path = directory.join(filename);
    let metadata = serde_json::json!({
        (crate::agent::codex::constants::RECORD_TYPE_KEY):
            crate::agent::codex::constants::SESSION_META_TYPE,
        (crate::agent::codex::constants::PAYLOAD_KEY): {
            (crate::agent::codex::constants::SESSION_ID_KEY): provider_session_id,
            (crate::agent::codex::constants::CWD_KEY): request.cwd,
            (crate::agent::codex::constants::SOURCE_KEY):
                crate::agent::codex::constants::CLI_SOURCE,
        }
    });
    std::fs::write(&path, format!("{metadata}{JSON_LINE_ENDING}")).unwrap();
    path
}
