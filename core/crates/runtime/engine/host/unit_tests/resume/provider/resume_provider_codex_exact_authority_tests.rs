use crate::host::tests::*;
fn codex_authority_arguments(arguments: &[&str]) -> Vec<String> {
    std::iter::once(crate::agent::codex::constants::EXECUTABLE)
        .chain(arguments.iter().copied())
        .map(str::to_owned)
        .collect()
}

fn codex_authority_membership(
    fixture: &ProviderWaitFixture,
    arguments: &[&str],
) -> ProviderRuntimeMembership {
    provider_membership_case(
        fixture,
        crate::agent::AgentKind::Codex,
        crate::agent::codex::constants::EXECUTABLE,
        codex_authority_arguments(arguments),
        Some(completion_provider_environment(&fixture.runtime)),
    )
}

#[test]
fn provider_authority_accepts_the_current_codex_emitter_dialect() {
    use crate::agent::codex::constants as codex;

    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let cwd = fixture.resumed_binding().cwd;
    assert!(matches!(
        codex_authority_membership(
            &fixture,
            &[
                codex::RESUME_ARGUMENT,
                codex::CWD_ARGUMENT,
                &cwd,
                COMPLETION_PROVIDER_SESSION_ID,
            ],
        ),
        ProviderRuntimeMembership::Verified(_)
    ));
}

#[test]
fn provider_authority_rejects_codex_resume_reinterpretation_attacks() {
    use crate::agent::codex::constants as codex;

    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    for arguments in [
        vec![
            codex::RESUME_ARGUMENT,
            codex::LAST_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
        ],
        vec![
            codex::RESUME_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
            codex::LAST_ARGUMENT,
        ],
        vec![
            codex::RESUME_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
            COMPLETION_PROVIDER_OTHER_SESSION_ID,
        ],
        vec![
            codex::RESUME_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
            codex::RESUME_ARGUMENT,
            COMPLETION_PROVIDER_OTHER_SESSION_ID,
        ],
        vec![
            codex::RESUME_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
            codex::FORK_ARGUMENT,
            COMPLETION_PROVIDER_OTHER_SESSION_ID,
        ],
        vec![
            codex::RESUME_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
            codex::CWD_ARGUMENT,
        ],
        vec![
            codex::RESUME_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
            CODEX_UNKNOWN_ARGUMENT,
        ],
        vec![
            codex::RESUME_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
            CODEX_AUTHORITY_RESUME_PROMPT,
        ],
        vec![
            codex::RESUME_ARGUMENT,
            CODEX_REMOTE_ENDPOINT_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
        ],
        vec![
            codex::RESUME_ARGUMENT,
            codex::VARIABLE_VALUE_ARGUMENTS[0],
            COMPLETION_PROVIDER_SESSION_ID,
        ],
    ] {
        assert_eq!(
            codex_authority_membership(&fixture, &arguments),
            ProviderRuntimeMembership::Contradicted
        );
    }
}
