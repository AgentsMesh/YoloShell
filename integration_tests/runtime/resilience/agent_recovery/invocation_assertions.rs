use super::*;

#[path = "codex_launch_diagnostics.rs"]
mod codex_launch_diagnostics;

const CLAUDE_RESUME_ARGUMENT_COUNT: usize = 2;
const CODEX_RESUME_ARGUMENT_COUNT: usize = 4;
const RESUME_COMMAND_INDEX: usize = 0;
const CODEX_CWD_ARGUMENT_INDEX: usize = 1;
const CODEX_CWD_INDEX: usize = 2;
const CODEX_SESSION_ID_INDEX: usize = 3;
const CLAUDE_SESSION_ID_INDEX: usize = 1;
pub(super) const INVOCATIONS_PER_RECOVERY: usize = 1;

pub(super) async fn assert_automatic_invocations(
    provider: ProviderFixture,
    data_home: &Path,
    fixtures: &[RecoveryFixture],
) {
    let recoveries = fixtures
        .iter()
        .filter(|fixture| matches!(fixture.expectation, AutomaticExpectation::Provider))
        .count();
    let invocations = wait_for_invocation_count(data_home, fixtures.len() + recoveries).await;
    for fixture in fixtures {
        let original_epoch = fixture
            .previous
            .runtime
            .as_ref()
            .expect("original runtime")
            .runtime_epoch
            .to_string();
        assert_eq!(
            invocations
                .iter()
                .filter(|record| {
                    record.runtime_epoch == original_epoch && record.arguments.is_empty()
                })
                .count(),
            INVOCATIONS_PER_RECOVERY
        );
        let resumes: Vec<_> = invocations
            .iter()
            .filter(|record| {
                is_resume(
                    record,
                    provider,
                    agent_id(&fixture.previous),
                    agent_cwd(&fixture.previous),
                )
            })
            .collect();
        match fixture.expectation {
            AutomaticExpectation::Provider => {
                assert_eq!(resumes.len(), INVOCATIONS_PER_RECOVERY);
                assert_ne!(resumes[0].runtime_epoch, original_epoch);
            }
            AutomaticExpectation::ExplicitOnly | AutomaticExpectation::FailClosed => {
                assert!(resumes.is_empty());
            }
        }
    }
}

pub(super) async fn assert_explicit_recovery_matrix(
    client: &Client,
    provider: ProviderFixture,
    fixtures: &[RecoveryFixture],
    data_home: &Path,
    harness: &Harness,
) {
    for fixture in fixtures {
        let session_id = fixture.previous.manifest.terminal_id;
        let automatic = client.get_session(session_id).await.unwrap();
        stop_automatic_runtime(client, fixture, &automatic, harness).await;
        let before = invocation_log(data_home);
        if matches!(fixture.expectation, AutomaticExpectation::FailClosed) {
            super::explicit_fail_closed::assert_recreate(
                client, session_id, data_home, &before, harness,
            )
            .await;
            continue;
        }
        if let Err(error) = resume_until_complete(harness, session_id, false).await {
            let diagnostics = codex_launch_diagnostics::capture(client, session_id, provider).await;
            let binding = read_json::<yolo_runtime::agent::AgentBinding>(
                &harness.paths().agent_binding(session_id),
            );
            panic!(
                "explicit recovery failed for {:?} with binding {binding:?}: {error:?}; {diagnostics}; host_stderr={}",
                fixture.state,
                harness.host_stderr_tail(),
            )
        }
        let previous_binding = fixture
            .previous
            .agent
            .as_ref()
            .expect("previous agent binding");
        let restored =
            wait_for_restored_binding(client, session_id, previous_binding, harness).await;
        assert_stable_binding(
            fixture.previous.agent.as_ref(),
            restored.agent.as_ref(),
            provider,
            data_home,
        );
        match fixture.expectation {
            AutomaticExpectation::Provider => {
                assert_recovered_identities_rotated(&automatic, &restored);
            }
            AutomaticExpectation::ExplicitOnly => {
                assert_runtime_views_rotated(&fixture.previous, &restored)
            }
            AutomaticExpectation::FailClosed => unreachable!(),
        }
        let after =
            wait_for_invocation_count(data_home, before.len() + INVOCATIONS_PER_RECOVERY).await;
        let invocation = after.last().unwrap();
        assert!(is_resume(
            invocation,
            provider,
            agent_id(&fixture.previous),
            agent_cwd(&fixture.previous),
        ));
        assert_eq!(
            invocation.runtime_epoch,
            restored.runtime.as_ref().unwrap().runtime_epoch.to_string()
        );
    }
}

fn is_resume(
    invocation: &fake_agent_invocation::Invocation,
    provider: ProviderFixture,
    session_id: &str,
    cwd: &str,
) -> bool {
    if invocation
        .arguments
        .get(RESUME_COMMAND_INDEX)
        .map(String::as_str)
        != Some(provider.resume_argument)
    {
        return false;
    }
    match provider.resume_cwd_argument {
        Some(cwd_argument) => {
            invocation.arguments.len() == CODEX_RESUME_ARGUMENT_COUNT
                && invocation.arguments[CODEX_CWD_ARGUMENT_INDEX] == cwd_argument
                && invocation.arguments[CODEX_CWD_INDEX] == cwd
                && invocation.arguments[CODEX_SESSION_ID_INDEX] == session_id
        }
        None => {
            invocation.arguments.len() == CLAUDE_RESUME_ARGUMENT_COUNT
                && invocation.arguments[CLAUDE_SESSION_ID_INDEX] == session_id
        }
    }
}

fn agent_id(view: &TerminalView) -> &str {
    &view.agent.as_ref().expect("agent binding").provider_session_id
}

fn agent_cwd(view: &TerminalView) -> &str {
    &view.agent.as_ref().expect("agent binding").cwd
}
