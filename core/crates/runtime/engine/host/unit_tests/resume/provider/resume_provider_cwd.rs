use crate::host::tests::*;
use super::*;

#[test]
fn provider_resume_uses_detected_cwd_and_fences_shell_cwd_substitution() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    for kind in [
        crate::agent::AgentKind::Codex,
        crate::agent::AgentKind::ClaudeCode,
    ] {
        let manifest = fixture_manifest(directory.path());
        host.paths.ensure_session(manifest.terminal_id).unwrap();
        let mut binding = host_binding(manifest.terminal_id);
        binding.kind = kind;
        let selected = ResumeBinding::Provider(binding.clone());
        let launch = host
            .prepare_resume_launch(&manifest, &selected, ResumeIntent::Explicit, Uuid::new_v4())
            .unwrap();
        assert_eq!(launch.cwd, PathBuf::from(&binding.cwd));
        assert_ne!(launch.cwd, manifest.cwd);

        let transaction = ResumeTransaction::new(
            Uuid::new_v4(),
            ResumeIntent::Explicit,
            ResumeTarget::from_binding(&selected),
            &launch,
        )
        .unwrap();
        host.validate_transaction_launch(&manifest, &transaction, &launch)
            .unwrap();
        let mut substituted = launch;
        substituted.cwd = manifest.cwd.clone();
        let changed = retarget_resume_transaction(&transaction, substituted);
        assert!(
            host.validate_transaction_launch(&manifest, &changed, &changed.launch)
                .is_err()
        );
    }
}

#[test]
fn automatic_codex_resume_pins_the_detected_cwd_in_the_native_command() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let detected_cwd = directory.path().join("detected-codex-cwd");
    std::fs::create_dir(&detected_cwd).unwrap();
    let mut binding = host_binding(manifest.terminal_id);
    binding.cwd = detected_cwd.display().to_string();
    binding.state = crate::agent::AgentState::WaitingForInput;
    binding.process_pid = Some(LEGACY_TEST_CHILD_PID);
    binding.process_start_time = Some(LEGACY_TEST_PROCESS_START);
    let selected = ResumeBinding::Provider(binding.clone());

    let launch = host
        .prepare_resume_launch(
            &manifest,
            &selected,
            ResumeIntent::Automatic,
            Uuid::new_v4(),
        )
        .unwrap();
    let expected = format!(
        "{} {} {} {} {}{}",
        crate::agent::codex::constants::EXECUTABLE,
        crate::agent::codex::constants::RESUME_ARGUMENT,
        crate::agent::codex::constants::CWD_ARGUMENT,
        binding.cwd,
        binding.provider_session_id,
        crate::agent::constants::COMMAND_LINE_ENDING,
    );
    assert_eq!(launch.cwd, detected_cwd);
    assert_eq!(launch.initial_input, Some(expected.into_bytes()));
    assert_ne!(launch.cwd, manifest.cwd);
}
