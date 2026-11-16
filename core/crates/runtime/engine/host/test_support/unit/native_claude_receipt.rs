use crate::host::tests::*;
fn publish_native_claude_receipt(
    fixture: &ProviderWaitFixture,
    binding: &AgentBinding,
) -> crate::agent::claude::generation::native::ClaudeNativeGeneration {
    use crate::agent::claude::generation::native::ClaudeNativeGenerationStore;
    use crate::worker::claude_hooks::*;
    const PROJECT: &str = "native-receipt-project";
    let home = Path::new(&binding.data_home);
    let transcript = home
        .join(crate::agent::claude::constants::EVIDENCE_DIR)
        .join(PROJECT)
        .join(&binding.provider_session_id)
        .with_extension(crate::agent::constants::JSONL_EXTENSION);
    std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    let metadata = serde_json::json!({"type": "user", "sessionId": binding.provider_session_id, "cwd": binding.cwd});
    std::fs::write(&transcript, format!("{metadata}\n")).unwrap();
    let evidence = crate::agent::claude::ClaudeAdapter
        .inspect_authoritative_evidence_with_hook(&transcript, Path::new(&binding.cwd), home, || {})
        .unwrap()
        .unwrap();
    let request = ClaudeNativeRequest {
        schema_version: CLAUDE_NATIVE_SCHEMA_VERSION,
        scope: ClaudeNativeScope {
            terminal_id: binding.terminal_id,
            runtime_epoch: fixture.runtime.runtime_epoch,
            agent: ClaudeProcessIdentity {
                pid: binding.process_pid.unwrap(),
                start_time: binding.process_start_time.unwrap(),
            },
        },
        event_sequence: INITIAL_CLAUDE_EVENT_SEQUENCE,
        event: ClaudeLifecycleEvent::SessionStart {
            event_id: Uuid::new_v4(),
            provider_session_id: binding.provider_session_id.clone(),
            source: ClaudeSessionStartSource::Resume,
            cwd: binding.cwd.clone().into(),
            transcript_path: transcript,
            state: ClaudeHookState::Running,
        },
    };
    let store = ClaudeNativeGenerationStore::from_binding_path(&fixture.binding_path()).unwrap();
    store
        .apply_in_namespace(
            &fixture.host.namespace,
            &request,
            crate::agent::authority::AuthorityProcessIdentity {
                pid: fixture.runtime.worker_pid,
                start_time: fixture.runtime.worker_start_time,
            },
            &fixture.runtime.boot_id,
            &evidence,
            || Ok(()),
        )
        .unwrap()
}
