use crate::host::tests::*;
const SHADOW_CODEX_PATH: &str = "/private/yolo/codex/provider-executable";

#[test]
fn provider_authority_rejects_retired_shadow_executables() {
    let fixture = ProviderWaitFixture::new(PROVIDER_WAIT_TEST_TIMEOUT);
    let binding = fixture.resumed_binding();
    let arguments = [
        SHADOW_CODEX_PATH,
        crate::agent::codex::constants::RESUME_ARGUMENT,
        crate::agent::codex::constants::CWD_ARGUMENT,
        binding.cwd.as_str(),
        COMPLETION_PROVIDER_SESSION_ID,
    ]
    .map(str::to_owned)
    .to_vec();
    let mut process = completion_provider_process(
        &binding,
        &fixture.runtime,
        arguments,
        Some(completion_provider_environment(&fixture.runtime)),
    );
    process.executable = SHADOW_CODEX_PATH.to_owned();
    assert_eq!(
        provider_membership_from_processes(
            &crate::agent::builtin_agent_registry(),
            &fixture.transaction,
            &fixture.runtime,
            &binding,
            &[process],
            binding.updated_at_ms,
        ),
        ProviderRuntimeMembership::Contradicted,
    );
}
