use super::test_support::{Fixture, MODEL, PERMISSION, SECOND_STAMP, SESSION_ID};
use super::*;

#[test]
fn codex_exec_session_evidence_satisfies_later_callbacks() {
    let mut fixture = Fixture::new();
    let path = fixture.write_rollout_with_source(
        SESSION_ID,
        SECOND_STAMP,
        crate::agent::codex::constants::EXEC_SOURCE,
    );
    fixture.agent.evidence_paths = Some(vec![path.clone()]);
    fixture.request.input = crate::codex_hooks::CodexHookInput::UserPromptSubmit {
        session_id: SESSION_ID.to_owned(),
        transcript_path: Some(path.clone()),
        cwd: fixture.request.cwd().to_path_buf(),
        model: MODEL.to_owned(),
        permission_mode: PERMISSION.to_owned(),
        turn_id: uuid::Uuid::new_v4().to_string(),
        prompt: "hi".to_owned(),
        agent_id: None,
        agent_type: None,
    };
    let (context, proof) = capture(&fixture.agent, &fixture.request, &fixture.policy).unwrap();
    let proof = proof.expect("verified exec rollout evidence");
    assert_eq!(proof.evidence().path, path);
    assert_eq!(context.cwd, fixture.request.cwd());
}
