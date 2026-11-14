use crate::host::tests::*;
struct CompletionProviderProcess {
    child: std::process::Child,
    identity: process::ProcessIdentity,
}

impl CompletionProviderProcess {
    fn spawn() -> Self {
        let child = spawn_resume_test_child();
        let identity = process::ProcessIdentity::capture(child.id()).unwrap();
        Self { child, identity }
    }

    fn agent(&self) -> process::ProcessIdentity {
        self.identity
    }
}

impl Drop for CompletionProviderProcess {
    fn drop(&mut self) {
        stop_resume_test_child(&mut self.child);
    }
}

fn completion_provider_environment(
    runtime: &RuntimeRecord,
) -> std::collections::BTreeMap<String, String> {
    [
        (
            crate::agent::constants::YOLO_TERMINAL_ID_ENV.to_owned(),
            runtime.terminal_id.hyphenated().to_string(),
        ),
        (
            crate::agent::constants::YOLO_RUNTIME_EPOCH_ENV.to_owned(),
            runtime.runtime_epoch.hyphenated().to_string(),
        ),
    ]
    .into_iter()
    .collect()
}

fn provider_arguments(executable: &str, resume: &str, provider_session_id: &str) -> Vec<String> {
    [executable, resume, provider_session_id]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn completion_provider_process(
    binding: &AgentBinding,
    runtime: &RuntimeRecord,
    argv: Vec<String>,
    environment: Option<std::collections::BTreeMap<String, String>>,
) -> crate::agent::ObservedProcess {
    let (pid, start_time) = binding.process_pid.zip(binding.process_start_time).unwrap();
    crate::agent::ObservedProcess {
        pid,
        start_time,
        parent_pid: runtime.child_pid,
        process_group: COMPLETION_PROVIDER_PROCESS_GROUP,
        foreground_group: COMPLETION_PROVIDER_PROCESS_GROUP,
        executable: binding.executable.clone(),
        argv,
        environment,
        cwd: Some(PathBuf::from(&binding.cwd)),
        evidence_paths: Some(vec![PathBuf::from(&binding.evidence_path)]),
    }
}
