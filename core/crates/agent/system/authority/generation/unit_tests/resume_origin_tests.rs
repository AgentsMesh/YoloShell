use super::fixture::{Fixture, SESSION};
use crate::{EventKind, GenerationRequest, GenerationScope, SessionStartSource};
use yolo_agent_authority_contract::AuthorityProcessIdentity;

fn resumed_request(fixture: &Fixture, event: EventKind) -> GenerationRequest {
    let mut request = fixture.request(1, event, SESSION, &fixture.transcript);
    request.scope = GenerationScope {
        terminal_id: fixture.scope.terminal_id,
        runtime_epoch: uuid::Uuid::new_v4(),
        agent: AuthorityProcessIdentity {
            pid: 43,
            start_time: 8,
        },
    };
    request
}

fn apply_in_new_runtime(
    fixture: &Fixture,
    request: &GenerationRequest,
) -> crate::HookGenerationRecord {
    fixture
        .store
        .apply_in_namespace(
            &fixture.namespace,
            request,
            AuthorityProcessIdentity {
                pid: 85,
                start_time: 10,
            },
            "next-boot",
            &fixture.evidence(&fixture.transcript),
            || Ok(()),
        )
        .unwrap()
}

#[test]
fn resume_in_a_new_runtime_inherits_the_durable_binding_origin() {
    let fixture = Fixture::new();
    let started = fixture.apply(&fixture.start()).unwrap();
    let request = resumed_request(
        &fixture,
        EventKind::SessionStart {
            source: SessionStartSource::Resume,
        },
    );
    let resumed = apply_in_new_runtime(&fixture, &request);
    assert_ne!(resumed.worker, started.worker);
    assert_eq!(resumed.binding.detected_at_ms, started.binding.detected_at_ms);
    assert_eq!(resumed.binding.process_pid, Some(43));
    assert!(resumed.authorizes_resume(&resumed.binding));
    assert_eq!(
        resumed.binding.recovery_availability,
        yolo_agent_contract::AgentRecoveryAvailability::Available
    );
}

#[test]
fn a_fresh_start_in_a_new_runtime_keeps_its_own_detection_time() {
    let fixture = Fixture::new();
    let started = fixture.apply(&fixture.start()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(crate::constants::DETECTION_GAP_MS));
    let request = resumed_request(
        &fixture,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    let fresh = apply_in_new_runtime(&fixture, &request);
    assert!(fresh.binding.detected_at_ms > started.binding.detected_at_ms);
}

#[test]
fn a_resume_over_a_revoked_authority_does_not_inherit_the_origin() {
    let fixture = Fixture::new();
    let started = fixture.apply(&fixture.start()).unwrap();
    fixture
        .store
        .revoke_in_namespace(&fixture.namespace, fixture.scope.terminal_id, || Ok(()))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(crate::constants::DETECTION_GAP_MS));
    let request = resumed_request(
        &fixture,
        EventKind::SessionStart {
            source: SessionStartSource::Resume,
        },
    );
    let revived = apply_in_new_runtime(&fixture, &request);
    assert!(revived.binding.detected_at_ms > started.binding.detected_at_ms);
}
