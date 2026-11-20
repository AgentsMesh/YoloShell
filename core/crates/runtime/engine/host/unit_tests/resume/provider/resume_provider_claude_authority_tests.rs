use crate::host::tests::*;
#[test]
fn provider_authority_rejects_non_resume_claude_identity_modes() {
    use crate::agent::claude::constants as claude;

    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let executable = claude::EXECUTABLE.to_owned();
    let provider_session_id = COMPLETION_PROVIDER_SESSION_ID.to_owned();
    let cases = [
        vec![
            executable.clone(),
            claude::SESSION_ID_ARGUMENT.to_owned(),
            provider_session_id.clone(),
        ],
        vec![executable.clone(), claude::CONTINUE_ARGUMENT.to_owned()],
        vec![
            executable.clone(),
            claude::RESUME_ARGUMENT.to_owned(),
            provider_session_id.clone(),
            claude::FORK_SESSION_ARGUMENT.to_owned(),
        ],
        vec![
            executable.clone(),
            claude::RESUME_ARGUMENT.to_owned(),
            provider_session_id.clone(),
            claude::RESUME_ARGUMENT.to_owned(),
            provider_session_id,
        ],
    ];
    for argv in cases {
        assert_eq!(
            provider_membership_case(
                &fixture,
                crate::agent::AgentKind::ClaudeCode,
                claude::EXECUTABLE,
                argv,
                Some(completion_provider_environment(&fixture.runtime)),
            ),
            ProviderRuntimeMembership::Contradicted
        );
    }
}
