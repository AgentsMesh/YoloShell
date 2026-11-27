use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_codex_after_reattachment_uses_the_same_opted_in_shell() {
    let home = tempfile::tempdir().unwrap();
    let base = home.path().canonicalize().unwrap();
    prepare_host_home(&base);
    let executable = bazel_runfile(HOST_BINARY_RUNFILE).expect(TEST_RUNTIME_MISSING);
    let config = base.join(MANUAL_CODEX_CONFIG);
    let installed = Command::new(&executable)
        .args(MANUAL_HOOK_INSTALL)
        .arg(&config)
        .status()
        .unwrap();
    assert!(installed.success());

    let bin = base.join(MANUAL_BIN);
    std::fs::create_dir(&bin).unwrap();
    let program = bin.join(MANUAL_PROVIDER);
    std::fs::write(&program, MANUAL_CODEX_PROGRAM).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(SHELL_MODE)).unwrap();
    let trace = base.join(MANUAL_CODEX_TRACE);
    let mut command = Command::new(executable);
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .args([cli::SERVE_COMMAND, cli::HOME_ARGUMENT])
        .arg(&base)
        .env(SHELL_ENV, MANUAL_TEST_SHELL)
        .env(MANUAL_USER_HOME, &base)
        .env(MANUAL_ZDOTDIR, &base)
        .env_remove(MANUAL_EXPORTED_FUNCTION)
        .env(MANUAL_PROVIDER_HOME, config)
        .env(MANUAL_BIN_ENV, bin)
        .env(MANUAL_TRACE_ENV, &trace)
        .env(HISTFILE_ENV, NULL_FILE)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    let host = spawn_logged_host(&base, &mut command);
    let harness = Harness::start_with_host(home, host).await;
    let view = harness.create_cat(MANUAL_SESSION).await;
    let id = view.manifest.terminal_id;
    let target = runtime_ref_from_view(&view);
    let client = harness.client();
    let writer = client
        .acquire_writer(target, WRITER_NAME, None)
        .await
        .unwrap();
    client
        .input(
            target,
            writer.generation,
            &writer.writer_capability,
            MANUAL_PATH_COMMAND.to_vec(),
        )
        .await
        .unwrap();
    for count in [1, 2] {
        let attachment = client.attach(id, 0).await.unwrap();
        assert_eq!(
            Some(attachment.runtime_epoch()),
            view.runtime.as_ref().map(|record| record.runtime_epoch)
        );
        client
            .input(
                target,
                writer.generation,
                &writer.writer_capability,
                MANUAL_CODEX_COMMAND.to_vec(),
            )
            .await
            .unwrap();
        timeout(PROCESS_EXIT_TIMEOUT, async {
            loop {
                if std::fs::read_to_string(&trace)
                    .unwrap_or_default()
                    .lines()
                    .count()
                    >= count
                {
                    break;
                }
                tokio::time::sleep(MANUAL_POLL).await;
            }
        })
        .await
        .expect("manual Codex was not invoked");
        attachment.detach().await.unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(trace).unwrap(),
        MANUAL_EXPECTED_TRACE.repeat(2)
    );
    client
        .input(
            target,
            writer.generation,
            &writer.writer_capability,
            EXIT_COMMAND.to_vec(),
        )
        .await
        .unwrap();
    wait_for_exit(&client, id).await;
}
