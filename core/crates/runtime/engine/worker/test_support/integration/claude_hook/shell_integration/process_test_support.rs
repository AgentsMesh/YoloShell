use std::process::{Command, Output, Stdio};
use std::time::Duration;

const SHELL_PROCESS_OBSERVATION_WINDOW: Duration = Duration::from_millis(500);

pub(super) fn observed_output(command: &mut Command) -> Output {
    let child = crate::process::spawn_waitable_test_child(
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    );
    std::thread::sleep(SHELL_PROCESS_OBSERVATION_WINDOW);
    child.wait_with_output().unwrap()
}
