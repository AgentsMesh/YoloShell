use super::*;

const SUCCESSOR_WORKER_PID: u32 = WORKER_PID + 1;

#[test]
fn confirmed_exit_without_resume_proof_cannot_authorize_worker_replacement() {
    let fixture = Fixture::new();
    let mut exited = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    exited.binding.state = AgentState::Exited;
    exited.binding.process_pid = None;
    exited.binding.process_start_time = None;

    assert!(exited.validate().is_err());
    assert!(!exited.authorizes(&exited.binding, successor_runtime(&fixture)));
}

#[test]
fn live_authority_does_not_cross_the_worker_runtime_boundary() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);

    assert!(!active.authorizes(&active.binding, successor_runtime(&fixture)));
}

#[test]
fn verified_exit_authority_still_requires_the_exact_binding() {
    let fixture = Fixture::new();
    let mut exited = fixture.verified_authority(Generation::INITIAL.0);
    exited.binding.state = AgentState::Exited;
    exited.binding.process_pid = None;
    exited.binding.process_start_time = None;
    let mut forged = exited.binding.clone();
    forged.provider_session_id.push_str("-forged");

    assert!(!exited.authorizes(&forged, successor_runtime(&fixture)));
}

#[test]
fn verified_exit_constructor_preserves_recovery_and_checks_the_transcript() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    let mut binding = active.binding.clone();
    binding.state = AgentState::Exited;
    binding.process_pid = None;
    binding.process_start_time = None;
    assert!(
        CodexBindingAuthority::new(
            fixture.runtime,
            active.provider_process(),
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            active.version,
            active.status,
            binding.clone(),
        )
        .is_err()
    );
    let exited = CodexBindingAuthority::new_with_verified_resume_identity(
        fixture.runtime,
        active.provider_process(),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        active.version,
        active.status,
        binding,
        TEST_RESUME_BOOT,
    )
    .unwrap();
    exited.validate().unwrap();
    assert!(exited.authorizes(&exited.binding, fixture.runtime));
    assert!(!exited.authorizes(&exited.binding, successor_runtime(&fixture)));
    std::fs::remove_file(&exited.binding.evidence_path).unwrap();
    assert!(!exited.authorizes(&exited.binding, fixture.runtime));
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
