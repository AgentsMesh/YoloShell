use crate::host::tests::*;
fn provider_membership_case(
    fixture: &ProviderWaitFixture,
    kind: crate::agent::AgentKind,
    executable: &str,
    argv: Vec<String>,
    environment: Option<std::collections::BTreeMap<String, String>>,
) -> ProviderRuntimeMembership {
    let mut binding = fixture.resumed_binding();
    binding.kind = kind.clone();
    binding.executable = executable.to_owned();
    let mut transaction = fixture.transaction.clone();
    transaction.target = ResumeTarget::Provider {
        agent_kind: kind,
        provider_session_id: binding.provider_session_id.clone(),
        binding: binding.clone(),
    };
    let process = completion_provider_process(&binding, &fixture.runtime, argv, environment);
    let observed_at_ms = binding.updated_at_ms;
    provider_membership_from_processes(
        &crate::agent::builtin_agent_registry(),
        &transaction,
        &fixture.runtime,
        &binding,
        &[process],
        observed_at_ms,
    )
}

#[test]
fn provider_authority_verifies_exact_codex_and_claude_resume_sessions() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let environment = Some(completion_provider_environment(&fixture.runtime));
    let codex = provider_membership_case(
        &fixture,
        crate::agent::AgentKind::Codex,
        crate::agent::codex::constants::EXECUTABLE,
        provider_arguments(
            crate::agent::codex::constants::EXECUTABLE,
            crate::agent::codex::constants::RESUME_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
        ),
        environment.clone(),
    );
    let claude = provider_membership_case(
        &fixture,
        crate::agent::AgentKind::ClaudeCode,
        crate::agent::claude::constants::EXECUTABLE,
        provider_arguments(
            crate::agent::claude::constants::EXECUTABLE,
            crate::agent::claude::constants::RESUME_ARGUMENT,
            COMPLETION_PROVIDER_SESSION_ID,
        ),
        environment,
    );

    assert!(matches!(codex, ProviderRuntimeMembership::Verified(_)));
    assert!(matches!(claude, ProviderRuntimeMembership::Verified(_)));
}

#[test]
fn provider_authority_rejects_a_background_resume_process() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let binding = fixture.resumed_binding();
    let mut process = fixture.valid_process();
    process.foreground_group = COMPLETION_PROVIDER_BACKGROUND_GROUP;

    assert_eq!(
        provider_membership_from_processes(
            &crate::agent::builtin_agent_registry(),
            &fixture.transaction,
            &fixture.runtime,
            &binding,
            &[process],
            binding.updated_at_ms,
        ),
        ProviderRuntimeMembership::Contradicted
    );
}

#[test]
fn provider_authority_rejects_wrong_codex_and_claude_resume_sessions() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let environment = Some(completion_provider_environment(&fixture.runtime));
    for (kind, executable, resume) in [
        (
            crate::agent::AgentKind::Codex,
            crate::agent::codex::constants::EXECUTABLE,
            crate::agent::codex::constants::RESUME_ARGUMENT,
        ),
        (
            crate::agent::AgentKind::ClaudeCode,
            crate::agent::claude::constants::EXECUTABLE,
            crate::agent::claude::constants::RESUME_ARGUMENT,
        ),
    ] {
        assert_eq!(
            provider_membership_case(
                &fixture,
                kind,
                executable,
                provider_arguments(executable, resume, COMPLETION_PROVIDER_OTHER_SESSION_ID),
                environment.clone(),
            ),
            ProviderRuntimeMembership::Contradicted
        );
    }
}

#[test]
fn provider_authority_rejects_non_exact_codex_resume_modes() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let environment = Some(completion_provider_environment(&fixture.runtime));
    let executable = crate::agent::codex::constants::EXECUTABLE;
    let resume = crate::agent::codex::constants::RESUME_ARGUMENT;
    let cases = [
        vec![executable.to_owned()],
        vec![executable.to_owned(), resume.to_owned()],
        provider_arguments(
            executable,
            resume,
            crate::agent::codex::constants::LAST_ARGUMENT,
        ),
        provider_arguments(executable, resume, COMPLETION_PROVIDER_INVALID_SESSION_ID),
    ];
    for argv in cases {
        assert_eq!(
            provider_membership_case(
                &fixture,
                crate::agent::AgentKind::Codex,
                executable,
                argv,
                environment.clone(),
            ),
            ProviderRuntimeMembership::Contradicted
        );
    }
}

#[test]
fn provider_authority_requires_canonical_yolo_environment() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let executable = crate::agent::codex::constants::EXECUTABLE;
    let argv = provider_arguments(
        executable,
        crate::agent::codex::constants::RESUME_ARGUMENT,
        COMPLETION_PROVIDER_SESSION_ID,
    );
    assert_eq!(
        provider_membership_case(
            &fixture,
            crate::agent::AgentKind::Codex,
            executable,
            argv.clone(),
            None,
        ),
        ProviderRuntimeMembership::NotObserved
    );
    for key in [
        crate::agent::constants::YOLO_TERMINAL_ID_ENV,
        crate::agent::constants::YOLO_RUNTIME_EPOCH_ENV,
    ] {
        let mut missing = completion_provider_environment(&fixture.runtime);
        missing.remove(key);
        assert_eq!(
            provider_membership_case(
                &fixture,
                crate::agent::AgentKind::Codex,
                executable,
                argv.clone(),
                Some(missing),
            ),
            ProviderRuntimeMembership::Contradicted
        );

        let mut environment = completion_provider_environment(&fixture.runtime);
        environment.insert(
            key.to_owned(),
            COMPLETION_PROVIDER_OTHER_SESSION_ID.to_owned(),
        );
        assert_eq!(
            provider_membership_case(
                &fixture,
                crate::agent::AgentKind::Codex,
                executable,
                argv.clone(),
                Some(environment),
            ),
            ProviderRuntimeMembership::Contradicted
        );
    }
}
