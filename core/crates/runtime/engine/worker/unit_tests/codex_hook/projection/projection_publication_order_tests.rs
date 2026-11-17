use super::*;
use crate::agent::authority::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::agent::codex::authority::{
    CodexAuthorityStatus, CodexAuthorityVersion, CodexBindingAuthority,
};
use crate::agent::hooks::{Generation, ProcessIdentity};
use crate::agent::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use crate::worker::codex_hook::projection::test_support::{ProjectionFixture, registry};
use uuid::Uuid;

const OLDER_ROUTE_REVISION: u64 = 1;
const NEWER_ROUTE_REVISION: u64 = OLDER_ROUTE_REVISION + 1;
const AUTHORITY_REVISION: u64 = 3;
const EVENT_SEQUENCE: u64 = 2;
const PROVIDER_PID: u32 = 41;
const PROVIDER_START_TIME: u64 = 42;
const PREDECESSOR_WORKER_PID: u32 = 51;
const SUCCESSOR_WORKER_PID: u32 = PREDECESSOR_WORKER_PID + 1;
const WORKER_START_TIME: u64 = 61;
const YOLO_SESSION_ID: u128 = 0x10000000000000000000000000000001;
const AGENT_SESSION_ID: &str = "0198f5ec-ec51-7f88-b0af-76f5bf7c3c11";
const CWD: &str = "/tmp/codex-projection-order";
const EVIDENCE_PATH: &str = "/tmp/codex-projection-order/rollout.jsonl";
const DATA_HOME: &str = "/tmp/codex-projection-order/codex-home";
const DETECTED_AT_MS: i64 = 100;
const UPDATED_AT_MS: i64 = 200;
const ACTIVE_LIFECYCLE_EXPECTED: &str = "active Codex lifecycle is unavailable";
const CONFIRMED_EXIT_EXPECTED: &str = "confirmed Codex exit lifecycle is unavailable";

#[test]
fn successor_runtime_cannot_publish_an_older_route() {
    let invocation = Uuid::new_v4();
    let current = authority(predecessor_runtime(), NEWER_ROUTE_REVISION, invocation);
    let stale = authority(successor_runtime(), OLDER_ROUTE_REVISION, Uuid::new_v4());

    assert!(publication_is_blocked(Some(&current), &stale));
}

#[test]
fn successor_runtime_can_take_over_the_same_durable_version() {
    let invocation = Uuid::new_v4();
    let current = authority(predecessor_runtime(), NEWER_ROUTE_REVISION, invocation);
    let successor = authority(successor_runtime(), NEWER_ROUTE_REVISION, invocation);

    assert!(!publication_is_blocked(Some(&current), &successor));
}

#[test]
fn unavailable_can_reactivate_but_revoked_cannot() {
    let invocation = Uuid::new_v4();
    let runtime = predecessor_runtime();
    let active = authority(runtime, NEWER_ROUTE_REVISION, invocation);
    let unavailable = active.unavailable_for_test().unwrap();
    let revoked = active.revoked_for_test().unwrap();
    let mut successor = authority(successor_runtime(), NEWER_ROUTE_REVISION, invocation);
    successor.binding.updated_at_ms = unavailable.binding.updated_at_ms;

    let mut same_runtime = active;
    same_runtime.binding.updated_at_ms = unavailable.binding.updated_at_ms;

    assert!(!publication_is_blocked(Some(&unavailable), &same_runtime));
    assert!(!publication_is_blocked(Some(&unavailable), &successor));
    assert!(publication_is_blocked(Some(&revoked), &successor));
}

#[test]
fn one_route_revision_cannot_name_another_invocation() {
    let current = authority(predecessor_runtime(), NEWER_ROUTE_REVISION, Uuid::new_v4());
    let conflict = authority(successor_runtime(), NEWER_ROUTE_REVISION, Uuid::new_v4());

    assert!(publication_is_blocked(Some(&current), &conflict));
}

#[test]
fn durable_same_version_binding_is_the_partial_commit_replay_base() {
    let fixture = ProjectionFixture::new();
    let route = fixture.route();
    fixture.start();
    let running_lifecycle = match fixture
        .authority
        .as_ref()
        .unwrap()
        .runtime
        .subscribe_authority()
        .borrow()
        .clone()
    {
        super::super::super::publication::CodexAuthorityView::Active(lifecycle) => lifecycle,
        _ => panic!("{ACTIVE_LIFECYCLE_EXPECTED}"),
    };
    let running = super::super::binding::project(
        &route,
        &running_lifecycle,
        running_lifecycle.authority_revision(),
        super::super::binding::ProjectionMode::Active,
        None,
        &registry(),
    )
    .unwrap();
    fixture.prompt();
    fixture.pre_compact();
    fixture.finish();
    fixture.confirm_exit();
    let lifecycle = match fixture
        .authority
        .as_ref()
        .unwrap()
        .runtime
        .subscribe_authority()
        .borrow()
        .clone()
    {
        super::super::super::publication::CodexAuthorityView::Active(lifecycle) => lifecycle,
        _ => panic!("{CONFIRMED_EXIT_EXPECTED}"),
    };
    let projected = super::super::binding::project(
        &route,
        &lifecycle,
        lifecycle.authority_revision(),
        super::super::binding::ProjectionMode::Active,
        Some(&running.binding),
        &registry(),
    )
    .unwrap();
    let current = CodexBindingAuthority::new_with_verified_resume_identity(
        predecessor_runtime(),
        projected.provider,
        projected.provider_version,
        projected.version,
        projected.status,
        projected.binding.clone(),
        &crate::process::boot_id(),
    )
    .unwrap();

    assert_eq!(
        replay_base(Some(&current), projected.version, Some(&running.binding)),
        Some(&projected.binding)
    );
}

fn authority(
    runtime: AuthorityRuntimeIdentity,
    route_revision: u64,
    invocation_id: Uuid,
) -> CodexBindingAuthority {
    CodexBindingAuthority::new(
        runtime,
        ProcessIdentity::new(PROVIDER_PID, PROVIDER_START_TIME),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        CodexAuthorityVersion {
            route_revision,
            invocation_id,
            generation: Generation::INITIAL,
            authority_revision: AUTHORITY_REVISION,
            sequence: EVENT_SEQUENCE,
        },
        CodexAuthorityStatus::Active,
        binding(),
    )
    .unwrap()
}

fn binding() -> AgentBinding {
    AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::from_u128(YOLO_SESSION_ID),
        kind: AgentKind::Codex,
        provider_session_id: AGENT_SESSION_ID.to_owned(),
        executable: crate::agent::codex::constants::EXECUTABLE.to_owned(),
        cwd: CWD.to_owned(),
        evidence_path: EVIDENCE_PATH.to_owned(),
        data_home: DATA_HOME.to_owned(),
        process_pid: Some(PROVIDER_PID),
        process_start_time: Some(PROVIDER_START_TIME),
        state: AgentState::Running,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: DETECTED_AT_MS,
        updated_at_ms: UPDATED_AT_MS,
    }
}

fn predecessor_runtime() -> AuthorityRuntimeIdentity {
    runtime(PREDECESSOR_WORKER_PID)
}

fn successor_runtime() -> AuthorityRuntimeIdentity {
    runtime(SUCCESSOR_WORKER_PID)
}

fn runtime(worker_pid: u32) -> AuthorityRuntimeIdentity {
    AuthorityRuntimeIdentity {
        runtime_epoch: Uuid::new_v4(),
        worker: AuthorityProcessIdentity {
            pid: worker_pid,
            start_time: WORKER_START_TIME,
        },
    }
}
