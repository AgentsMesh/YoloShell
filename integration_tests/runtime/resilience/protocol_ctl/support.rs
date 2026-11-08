use super::*;
use std::io::Write;

const PROCESS_INSPECTOR: &str = "/bin/ps";
const PROCESS_ID_ARGUMENT: &str = "-p";
const PROCESS_OUTPUT_ARGUMENT: &str = "-o";
const PROCESS_COMMAND_FORMAT: &str = "command=";

pub(super) fn run_ctl(
    harness: &Harness,
    arguments: &[&str],
    stdin: Option<&[u8]>,
) -> serde_json::Value {
    let mut command = Command::new(runtime_binary());
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .args([cli::CONTROL_COMMAND, cli::HOME_ARGUMENT])
        .arg(harness.home.path())
        .args(arguments);
    let output = if let Some(input) = stdin {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    } else {
        command.output().unwrap()
    };
    assert!(
        output.status.success(),
        "ctl {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: yolo_capnp_codec::Value = if arguments.first() == Some(&cli::ATTACH_COMMAND) {
        // Attach is the streaming command; its initial document has the same
        // explicit length frame as subsequent event documents.
        let mut remaining = output.stdout.as_slice();
        let value = read_frame_blocking(&mut remaining).unwrap();
        assert!(remaining.is_empty());
        value
    } else {
        yolo_capnp_codec::decode(&output.stdout).unwrap_or_else(|error| {
            panic!("ctl {arguments:?} returned an invalid private document: {error}")
        })
    };
    serde_json::to_value(value).unwrap()
}

pub(super) fn assert_ctl_arguments_hide_secrets(
    harness: &Harness,
    arguments: &[&str],
    secrets: &[String],
) {
    let mut child = Command::new(runtime_binary())
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .args([cli::CONTROL_COMMAND, cli::HOME_ARGUMENT])
        .arg(harness.home.path())
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let private_input = child.stdin.take().unwrap();
    let process_id = child.id().to_string();
    let inspected = Command::new(PROCESS_INSPECTOR)
        .args([
            PROCESS_OUTPUT_ARGUMENT,
            PROCESS_COMMAND_FORMAT,
            PROCESS_ID_ARGUMENT,
            &process_id,
        ])
        .output()
        .unwrap();
    assert!(inspected.status.success());
    let command = String::from_utf8(inspected.stdout).unwrap();
    assert!(command.contains(cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT));
    assert!(secrets.iter().all(|secret| !command.contains(secret)));
    drop(private_input);
    let output = child.wait_with_output().unwrap();
    assert!(
        !output.status.success(),
        "empty private input must fail closed"
    );
}
