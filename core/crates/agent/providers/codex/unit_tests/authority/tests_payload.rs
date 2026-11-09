use super::*;

const NEWER_ROUTE_REVISION: u64 = ROUTE_REVISION + 1;
const SUCCESSOR_WORKER_PID: u32 = WORKER_PID + 1;
const TIMESTAMP_INCREMENT: i64 = 1;
const FORGED_SESSION_ID: &str = "0198f5ec-ec51-7f88-b0af-76f5bf7c3c12";
const FORGED_CWD: &str = "/tmp/forged-codex-cwd";
const FORGED_EVIDENCE: &str = "/tmp/forged-codex-rollout.jsonl";

#[test]
fn same_version_and_status_require_an_exact_binding_replay() {
    let fixture = Fixture::new();
    let current = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&current).unwrap();
    let mut session = current.clone();
    session.binding.provider_session_id = FORGED_SESSION_ID.to_owned();
    let mut cwd = current.clone();
    cwd.binding.cwd = FORGED_CWD.to_owned();
    let mut evidence = current.clone();
    evidence.binding.evidence_path = FORGED_EVIDENCE.to_owned();
    let mut timestamp = current.clone();
    timestamp.binding.updated_at_ms += TIMESTAMP_INCREMENT;
    let mut detected = current.clone();
    detected.binding.detected_at_ms += TIMESTAMP_INCREMENT;

    for forged in [session, cwd, evidence, timestamp, detected] {
        assert!(fixture.publish(&forged).is_err());
        assert_eq!(
            fixture
                .store()
                .load_in_namespace(&fixture.namespace)
                .unwrap(),
            Some(current.clone())
        );
    }
}

#[test]
fn same_version_accepts_only_the_canonical_terminal_transformation() {
    let fixture = Fixture::new();
    let current = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&current).unwrap();
    let terminal = current.unavailable().unwrap();
    fixture.publish(&terminal).unwrap();

    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(terminal)
    );
}

#[test]
fn same_version_accepts_canonical_authority_exhaustion() {
    let fixture = Fixture::new();
    let current = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&current).unwrap();
    let exhausted = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Exhausted);

    fixture.publish(&exhausted).unwrap();
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(exhausted)
    );
}

#[test]
fn confirmed_exit_cannot_be_replaced_by_a_same_version_terminal_shape() {
    let fixture = Fixture::new();
    let mut exited = fixture.verified_authority(Generation::INITIAL.0);
    exited.binding.state = AgentState::Exited;
    exited.binding.process_pid = None;
    exited.binding.process_start_time = None;
    fixture.publish(&exited).unwrap();
    let terminal = exited.unavailable().unwrap();

    assert!(fixture.publish(&terminal).is_err());
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(exited)
    );
}

#[test]
fn terminal_transformation_cannot_mutate_binding_identity() {
    let fixture = Fixture::new();
    let current = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&current).unwrap();
    let mut forged = current.unavailable().unwrap();
    forged.binding.cwd = FORGED_CWD.to_owned();

    assert!(fixture.publish(&forged).is_err());
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(current)
    );
}

#[test]
fn successor_runtime_requires_a_newer_route_publication() {
    let fixture = Fixture::new();
    let mut current = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    current.version.route_revision = NEWER_ROUTE_REVISION;
    fixture.publish(&current).unwrap();
    let mut successor = reissue(&current, successor_runtime(&fixture));
    assert!(fixture.publish(&successor).is_err());
    successor.version.route_revision += NEWER_ROUTE_REVISION;
    fixture.publish(&successor).unwrap();
    let mut stale = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    stale = reissue(&stale, successor_runtime(&fixture));

    assert!(fixture.publish(&stale).is_err());
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(successor)
    );
}

#[test]
fn successor_runtime_cannot_reactivate_a_finalized_route_without_a_new_publication() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();
    let terminal = active.unavailable().unwrap();
    fixture.publish(&terminal).unwrap();
    let mut reactivated = reissue(&active, successor_runtime(&fixture));
    reactivated.binding.updated_at_ms = terminal.binding.updated_at_ms;

    assert!(fixture.publish(&reactivated).is_err());
    reactivated.version.route_revision = NEWER_ROUTE_REVISION;
    fixture.publish(&reactivated).unwrap();
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(reactivated)
    );
}

#[test]
fn canonical_reactivation_cannot_mutate_a_finalized_binding() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();
    let terminal = active.unavailable().unwrap();
    fixture.publish(&terminal).unwrap();
    let mut forged = reissue(&active, successor_runtime(&fixture));
    forged.binding.updated_at_ms = terminal.binding.updated_at_ms;
    forged.binding.evidence_path = FORGED_EVIDENCE.to_owned();

    assert!(fixture.publish(&forged).is_err());
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap(),
        Some(terminal)
    );
}

fn reissue(
    authority: &CodexBindingAuthority,
    runtime: AuthorityRuntimeIdentity,
) -> CodexBindingAuthority {
    CodexBindingAuthority::new(
        runtime,
        ProcessIdentity::new(PROVIDER_PID, PROVIDER_START),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        authority.version,
        authority.status,
        authority.binding.clone(),
    )
    .unwrap()
}

fn successor_runtime(fixture: &Fixture) -> AuthorityRuntimeIdentity {
    AuthorityRuntimeIdentity {
        runtime_epoch: Uuid::new_v4(),
        worker: AuthorityProcessIdentity {
            pid: SUCCESSOR_WORKER_PID,
            start_time: fixture.runtime.worker.start_time,
        },
    }
}
