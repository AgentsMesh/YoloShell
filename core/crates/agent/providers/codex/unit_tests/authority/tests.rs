use super::*;
use yolo_agent_authority_contract::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::agent::hooks::{Generation, ProcessIdentity};
use crate::agent::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use crate::namespace_lease::NamespaceLease;
use uuid::Uuid;

#[path = "tests_fixture_writer.rs"]
mod fixture_writer_tests;
#[path = "tests_invalidation_commit_fence.rs"]
mod invalidation_commit_fence_tests;
#[path = "tests_invalidation.rs"]
mod invalidation_tests;
#[path = "tests_payload.rs"]
mod payload_tests;
#[path = "tests_progression.rs"]
mod progression_tests;
#[path = "tests_reactivation.rs"]
mod reactivation_tests;
#[path = "tests_recovery.rs"]
mod recovery_tests;
#[path = "tests_repair.rs"]
mod repair_tests;
#[path = "tests_repair_unavailable.rs"]
mod repair_unavailable_tests;
#[path = "../../test_support/authority/resume_fixture.rs"]
mod resume_fixture;
#[path = "tests_resume_identity.rs"]
mod resume_identity_tests;
#[path = "tests_revocation.rs"]
mod revocation_tests;
#[path = "validation_matrix_tests.rs"]
mod validation_matrix_tests;
use resume_fixture::TEST_RESUME_BOOT;

const ROUTE_REVISION: u64 = 1;
const ACTIVE_SEQUENCE: u64 = 2;
const ACTIVE_AUTHORITY_REVISION: u64 = ACTIVE_SEQUENCE + 1;
const PROVIDER_PID: u32 = 41;
const PROVIDER_START: u64 = 42;
const WORKER_PID: u32 = 51;
const WORKER_START: u64 = 52;
const SESSION_ID: &str = "0198f5ec-ec51-7f88-b0af-76f5bf7c3c11";

struct Fixture {
    _directory: tempfile::TempDir,
    namespace: NamespaceLease,
    binding_path: std::path::PathBuf,
    runtime: AuthorityRuntimeIdentity,
    invocation_id: Uuid,
    terminal_id: Uuid,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        resume_fixture::write_evidence(directory.path());
        let session = directory.path().join("session");
        crate::paths::directory::PrivateDirectory::ensure(&session).unwrap();
        let namespace = NamespaceLease::capture(&[("codex authority test", &session)]).unwrap();
        Self {
            binding_path: session.join(crate::agent::constants::BINDING_FILE_NAME),
            runtime: AuthorityRuntimeIdentity {
                runtime_epoch: Uuid::new_v4(),
                worker: AuthorityProcessIdentity {
                    pid: WORKER_PID,
                    start_time: WORKER_START,
                },
            },
            invocation_id: Uuid::new_v4(),
            terminal_id: Uuid::new_v4(),
            _directory: directory,
            namespace,
        }
    }

    fn authority(&self, generation: u64, status: CodexAuthorityStatus) -> CodexBindingAuthority {
        let unavailable = status != CodexAuthorityStatus::Active;
        CodexBindingAuthority::new(
            self.runtime,
            ProcessIdentity::new(PROVIDER_PID, PROVIDER_START),
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            CodexAuthorityVersion {
                route_revision: ROUTE_REVISION,
                invocation_id: self.invocation_id,
                generation: Generation(generation),
                authority_revision: ACTIVE_AUTHORITY_REVISION,
                sequence: ACTIVE_SEQUENCE,
            },
            status,
            AgentBinding {
                schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
                terminal_id: self.terminal_id,
                kind: AgentKind::Codex,
                provider_session_id: SESSION_ID.to_owned(),
                executable: crate::codex::constants::EXECUTABLE.to_owned(),
                cwd: self
                    ._directory
                    .path()
                    .canonicalize()
                    .unwrap()
                    .display()
                    .to_string(),
                evidence_path: resume_fixture::evidence_path(self._directory.path())
                    .display()
                    .to_string(),
                data_home: self
                    ._directory
                    .path()
                    .canonicalize()
                    .unwrap()
                    .display()
                    .to_string(),
                process_pid: Some(PROVIDER_PID),
                process_start_time: Some(PROVIDER_START),
                state: if unavailable {
                    AgentState::Unknown
                } else {
                    AgentState::Running
                },
                recovery_availability: if unavailable {
                    AgentRecoveryAvailability::Unavailable
                } else {
                    AgentRecoveryAvailability::Available
                },
                detected_at_ms: 1,
                updated_at_ms: 2,
            },
        )
        .unwrap()
    }

    fn store(&self) -> CodexBindingAuthorityStore {
        CodexBindingAuthorityStore::from_binding_path(&self.binding_path).unwrap()
    }

    fn publish(&self, authority: &CodexBindingAuthority) -> anyhow::Result<()> {
        self.store().publish_in_namespace_with_commit_fence(
            &self.namespace,
            &self.binding_path,
            authority,
            || Ok(()),
        )
    }

    fn invalidate(
        &self,
        runtime: AuthorityRuntimeIdentity,
    ) -> anyhow::Result<Option<CodexBindingAuthority>> {
        self.store().invalidate_in_namespace_with_commit_fence(
            &self.namespace,
            &self.binding_path,
            runtime,
            || Ok(()),
        )
    }
}

#[test]
fn exact_active_authority_and_binding_publish_together() {
    let fixture = Fixture::new();
    let authority = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&authority).unwrap();

    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(authority.clone())
    );
    assert!(authority.authorizes(&authority.binding, fixture.runtime));
}

#[test]
fn stale_generation_cannot_replace_newer_authority() {
    let fixture = Fixture::new();
    let current = fixture.authority(Generation::INITIAL.0 + 1, CodexAuthorityStatus::Active);
    let stale = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&current).unwrap();

    assert!(fixture.publish(&stale).is_err());
}

#[test]
fn unavailable_authority_cannot_regress_to_exhausted_at_the_same_version() {
    let fixture = Fixture::new();
    let unavailable = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Unavailable);
    fixture.publish(&unavailable).unwrap();
    let regressed = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Exhausted);

    assert!(fixture.publish(&regressed).is_err());
}
