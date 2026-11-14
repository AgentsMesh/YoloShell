use super::*;

const RELEASE_FILE: &str = "first-hook-release";
const HOOK_GATE_MODE: libc::mode_t = 0o600;
const HOOK_GATE_FAILURE: &str = "native hook gate reader did not become available";
const WRITER: &str = "native-registration-race";
const RELEASED: &[u8] = b"released\n";
const SINGLE_INVOCATION: usize = 1;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn first_native_hook_replaces_unmanaged_observation_without_relaunching_agent() {
    let home = tempdir().unwrap();
    let paths = Paths::new(home.path());
    paths.ensure_base().unwrap();
    let bin = home.path().join(AGENT_BIN_DIR);
    std::fs::create_dir_all(&bin).unwrap();
    install_fake_agent(CODEX_PROVIDER, &bin.join(CODEX_PROVIDER.executable));
    let host = spawn_agent_host(home.path(), CODEX_PROVIDER, &bin);
    let harness = Harness::start_with_host(home, host).await;
    let data_home = harness.home.path().join(CODEX_DATA_HOME_DIRECTORY);
    let release = harness.home.path().join(RELEASE_FILE);
    pause_first_hook(&data_home, &release);
    let client = harness.client();
    let created = harness.create_cat(WRITER).await;
    let target = runtime_ref_from_view(&created);
    let lease = client.acquire_writer(target, WRITER, None).await.unwrap();
    let home = quote(&data_home);
    let command = format!(
        "export {}='{home}' && {}{COMMAND_LINE_ENDING}",
        CODEX_PROVIDER.data_home_environment, CODEX_PROVIDER.executable,
    );
    client
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            command.into_bytes(),
        )
        .await
        .unwrap();
    let observed = wait_for_view(
        &client,
        created.manifest.terminal_id,
        |view| {
            view.agent.as_ref().is_some_and(|binding| {
                binding.recovery_availability
                    == yolo_runtime::agent::AgentRecoveryAvailability::UnmanagedLaunch
            })
        },
        Some(&harness),
        &harness,
    )
    .await;
    let before = observed.agent.unwrap();
    release_first_hook(&release).await;
    let registered = wait_for_binding(&client, created.manifest.terminal_id, &harness).await;
    let after = registered.agent.unwrap();
    assert_eq!(after.process_pid, before.process_pid);
    assert_eq!(after.process_start_time, before.process_start_time);
    assert_eq!(after.provider_session_id, before.provider_session_id);
    assert_eq!(after.evidence_path, before.evidence_path);
    assert_eq!(
        after.recovery_availability,
        yolo_runtime::agent::AgentRecoveryAvailability::Available
    );
    assert_eq!(invocation_log(&data_home).len(), SINGLE_INVOCATION);
}

fn pause_first_hook(home: &Path, release: &Path) {
    use fake_agent_constants::{
        CODEX_SESSION_START_EVENT, PACKAGED_SETTINGS_COMMAND_FIELD, PACKAGED_SETTINGS_HOOKS_FIELD,
    };
    use std::os::unix::ffi::OsStrExt;
    let gate = std::ffi::CString::new(release.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifo(gate.as_ptr(), HOOK_GATE_MODE) },
        libc::c_int::default()
    );
    let preview =
        yolo_hook_installation::inspect(&yolo_hook_installation_catalog::CODEX, home).unwrap();
    let mut document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&preview.path).unwrap()).unwrap();
    let groups = document[PACKAGED_SETTINGS_HOOKS_FIELD][CODEX_SESSION_START_EVENT]
        .as_array_mut()
        .unwrap();
    for group in groups {
        for hook in group[PACKAGED_SETTINGS_HOOKS_FIELD].as_array_mut().unwrap() {
            let command = hook[PACKAGED_SETTINGS_COMMAND_FIELD].as_str().unwrap();
            hook[PACKAGED_SETTINGS_COMMAND_FIELD] = serde_json::Value::String(format!(
                "IFS= read -r yolo_hook_gate < '{}'; {command}",
                quote(release),
            ));
        }
    }
    std::fs::write(preview.path, serde_json::to_vec(&document).unwrap()).unwrap();
}

async fn release_first_hook(path: &Path) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let deadline = tokio::time::Instant::now() + AGENT_STATE_SETTLEMENT_TIMEOUT;
    loop {
        let opened = std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path);
        match opened {
            Ok(mut writer) => {
                writer.write_all(RELEASED).unwrap();
                return;
            }
            Err(error)
                if error.raw_os_error() == Some(libc::ENXIO)
                    && tokio::time::Instant::now() < deadline =>
            {
                tokio::time::sleep(POLL_INTERVAL).await;
            }
            Err(error) => panic!("{HOOK_GATE_FAILURE}: {error}"),
        }
    }
}

fn quote(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\'', SHELL_SINGLE_QUOTE_ESCAPE)
}
