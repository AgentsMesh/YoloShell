use super::test_support::{FOREIGN_SESSION_ID, Fixture, SECOND_STAMP, SESSION_ID};
use super::*;

#[test]
fn nullable_transcript_uses_only_the_unique_observed_exact_rollout() {
    let fixture = Fixture::new();
    let (context, proof) = capture(&fixture.agent, &fixture.request, &fixture.policy).unwrap();
    let proof = proof.expect("verified rollout evidence");
    assert_eq!(proof.evidence().path, fixture.path);
    assert_eq!(context.cwd, fixture.request.cwd());
    let admitted = super::super::NativeAdmission {
        agent: fixture.agent.clone(),
        context: context.clone(),
        lineage: Vec::new(),
        evidence: Some(proof),
    };
    assert_eq!(
        admitted
            .envelope(&fixture.request, &context.identity)
            .unwrap()
            .metadata
            .transcript_path,
        Some(fixture.path),
    );
    assert!(
        fixture
            .request
            .envelope(&context.identity)
            .unwrap()
            .metadata
            .transcript_path
            .is_none()
    );
}

#[test]
fn explicit_verified_path_is_preserved_in_the_internal_event() {
    let mut fixture = Fixture::new();
    fixture.explicit_path(fixture.path.clone());
    let (context, proof) = capture(&fixture.agent, &fixture.request, &fixture.policy).unwrap();
    let proof = proof.expect("verified rollout evidence");
    let admitted = super::super::NativeAdmission {
        agent: fixture.agent,
        context: context.clone(),
        lineage: Vec::new(),
        evidence: Some(proof),
    };
    assert_eq!(
        admitted
            .envelope(&fixture.request, &context.identity)
            .unwrap(),
        fixture.request.envelope(&context.identity).unwrap(),
    );
}

#[test]
fn duplicate_descriptors_for_the_same_rollout_do_not_create_ambiguity() {
    let mut fixture = Fixture::new();
    fixture
        .agent
        .evidence_paths
        .as_mut()
        .unwrap()
        .push(fixture.path.clone());
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_ok());
}

#[test]
fn absent_or_foreign_observed_evidence_is_not_replaced_by_a_directory_scan() {
    let mut fixture = Fixture::new();
    fixture.agent.evidence_paths = None;
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
    fixture.agent.evidence_paths = Some(Vec::new());
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
    let foreign = fixture.write_rollout(FOREIGN_SESSION_ID, SECOND_STAMP);
    fixture.agent.evidence_paths = Some(vec![foreign]);
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
}

#[test]
fn two_observed_rollouts_for_one_session_are_rejected_as_ambiguous() {
    let mut fixture = Fixture::new();
    let second = fixture.write_rollout(SESSION_ID, SECOND_STAMP);
    fixture.agent.evidence_paths.as_mut().unwrap().push(second);
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
    fixture.explicit_path(fixture.path.clone());
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_ok());
}

#[test]
fn explicit_forged_transcript_does_not_fall_back_to_other_observed_evidence() {
    let mut fixture = Fixture::new();
    let foreign = fixture.write_rollout(FOREIGN_SESSION_ID, SECOND_STAMP);
    fixture.explicit_path(foreign.clone());
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
    fixture.agent.evidence_paths.as_mut().unwrap().push(foreign);
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
}

#[test]
fn cross_home_and_cross_runtime_claims_are_rejected() {
    let mut fixture = Fixture::new();
    let foreign = Fixture::new();
    fixture.request.data_home_candidates = vec![foreign.data_home];
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
    fixture.request.data_home_candidates = vec![fixture.data_home];
    fixture.agent.environment.as_mut().unwrap().insert(
        crate::codex_hooks::RUNTIME_ENV.to_owned(),
        uuid::Uuid::new_v4().to_string(),
    );
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
}

#[test]
fn session_start_before_the_rollout_exists_is_admitted_provisionally() {
    let mut fixture = Fixture::new();
    let pending = fixture.rollout_path(SESSION_ID, SECOND_STAMP);
    assert!(!pending.exists());
    fixture.explicit_path(pending.clone());
    fixture.agent.evidence_paths = Some(Vec::new());
    let (context, proof) = capture(&fixture.agent, &fixture.request, &fixture.policy).unwrap();
    assert!(proof.is_none());
    assert_eq!(context.cwd, fixture.request.cwd());
    let admitted = super::super::NativeAdmission {
        agent: fixture.agent,
        context: context.clone(),
        lineage: Vec::new(),
        evidence: proof,
    };
    assert_eq!(
        admitted
            .envelope(&fixture.request, &context.identity)
            .unwrap()
            .metadata
            .transcript_path,
        Some(pending)
    );
}

#[test]
fn session_start_with_a_created_but_unauthoritative_rollout_is_provisional() {
    let mut fixture = Fixture::new();
    let pending = fixture.write_pending_rollout(SESSION_ID, SECOND_STAMP);
    fixture.explicit_path(pending.clone());
    fixture.agent.evidence_paths = Some(vec![pending]);
    let (context, proof) = capture(&fixture.agent, &fixture.request, &fixture.policy).unwrap();
    assert!(proof.is_none());
    assert_eq!(context.cwd, fixture.request.cwd());
}

#[test]
fn session_start_still_rejects_a_transcript_for_another_session() {
    let mut fixture = Fixture::new();
    let foreign = fixture.rollout_path(FOREIGN_SESSION_ID, SECOND_STAMP);
    fixture.explicit_path(foreign);
    fixture.agent.evidence_paths = Some(Vec::new());
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
}

#[test]
fn session_start_verifies_a_written_rollout_without_an_observed_descriptor() {
    let mut fixture = Fixture::new();
    fixture.explicit_path(fixture.path.clone());
    fixture.agent.evidence_paths = Some(Vec::new());
    let (context, proof) = capture(&fixture.agent, &fixture.request, &fixture.policy).unwrap();
    let proof = proof.expect("verified rollout evidence");
    assert_eq!(proof.evidence().path, fixture.path);
    assert_eq!(context.cwd, fixture.request.cwd());
}

#[test]
fn session_start_without_an_announced_transcript_is_rejected() {
    let mut fixture = Fixture::new();
    fixture.agent.evidence_paths = Some(Vec::new());
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
}

#[test]
fn later_callbacks_require_written_transcript_evidence() {
    let mut fixture = Fixture::new();
    let pending = fixture.rollout_path(SESSION_ID, SECOND_STAMP);
    fixture.agent.evidence_paths = Some(Vec::new());
    fixture.request.input = crate::codex_hooks::CodexHookInput::UserPromptSubmit {
        session_id: SESSION_ID.to_owned(),
        transcript_path: Some(pending),
        cwd: fixture.request.cwd().to_path_buf(),
        model: "gpt-5".to_owned(),
        permission_mode: "default".to_owned(),
        turn_id: uuid::Uuid::new_v4().to_string(),
        prompt: "hi".to_owned(),
        agent_id: None,
        agent_type: None,
    };
    assert!(capture(&fixture.agent, &fixture.request, &fixture.policy).is_err());
}
