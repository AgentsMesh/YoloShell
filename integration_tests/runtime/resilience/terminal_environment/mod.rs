use super::*;
use std::os::unix::fs::PermissionsExt;

mod constants;
use self::constants::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn locale_free_host_creates_a_shell_that_reads_chinese_without_meta_commands() {
    let home = tempfile::tempdir().unwrap();
    let host = locale_free_host(home.path());
    let harness = Harness::start_with_host(home, host).await;
    let session = harness.create_cat(SESSION_NAME).await;
    let id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let client = harness.client();
    read_until(client.attach(id, 0).await.unwrap(), SHELL_PROMPT.as_bytes()).await;
    let writer = client
        .acquire_writer(target, WRITER_NAME, None)
        .await
        .unwrap();

    client
        .input(
            target,
            writer.generation,
            &writer.writer_capability,
            LOCALE_COMMAND.to_vec(),
        )
        .await
        .unwrap();
    let expected_locale = format!("YOLO_LOCALE:unset:{EXPECTED_LOCALE}:unset");
    read_until(
        client.attach(id, 0).await.unwrap(),
        expected_locale.as_bytes(),
    )
    .await;

    client
        .input(
            target,
            writer.generation,
            &writer.writer_capability,
            READ_COMMAND.to_vec(),
        )
        .await
        .unwrap();
    read_until(client.attach(id, 0).await.unwrap(), READ_READY).await;
    client
        .input(
            target,
            writer.generation,
            &writer.writer_capability,
            IME_INPUT.as_bytes().to_vec(),
        )
        .await
        .unwrap();
    let output = read_until(client.attach(id, 0).await.unwrap(), IME_RESULT.as_bytes()).await;
    assert!(
        !output
            .windows(READLINE_NUMERIC_ARGUMENT.len())
            .any(|part| part == READLINE_NUMERIC_ARGUMENT)
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

fn locale_free_host(home: &Path) -> SpawnedHost {
    prepare_host_home(home);
    let shell = home.join(SHELL_FILE);
    std::fs::write(&shell, SHELL_CONTENT).unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(SHELL_MODE)).unwrap();
    let codex_source = home.join(CODEX_SOURCE);
    std::fs::create_dir(&codex_source).unwrap();
    std::fs::write(codex_source.join(CODEX_CONFIG_FILE), CODEX_CONFIG).unwrap();
    let executable = bazel_runfile(HOST_BINARY_RUNFILE).expect(TEST_RUNTIME_MISSING);
    let mut command = Command::new(executable);
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .args([cli::SERVE_COMMAND, cli::HOME_ARGUMENT])
        .arg(home)
        .env(SHELL_ENV, shell)
        .env(CODEX_HOME_ENV, codex_source)
        .env(INPUTRC_ENV, NULL_FILE)
        .env(HISTFILE_ENV, NULL_FILE)
        .env(PROMPT_ENV, SHELL_PROMPT)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    for key in LOCALE_KEYS {
        command.env_remove(key);
    }
    spawn_logged_host(home, &mut command)
}
