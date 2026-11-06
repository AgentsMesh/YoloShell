use super::{
    DetachedStandardError, spawn_detached_before_deadline, spawn_detached_with_private_input,
    waitable_child_spawn_guard,
};
use std::path::Path;
use std::time::{Duration, Instant};

const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const CAPTURE_SCRIPT: &str = "printf spawned > \"$1\"";
const SHELL_ARGUMENT_ZERO: &str = "yolo-deadline-fence";
const CAPTURE_FILE: &str = "spawned";
const GATE_WAIT_BUDGET: Duration = Duration::from_millis(10);
const PRIVATE_INPUT: &[u8] = b"private";

#[test]
fn contended_spawn_gate_prevents_plain_and_private_spawns_at_deadline() {
    for private in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let capture = root.path().join(CAPTURE_FILE);
        let arguments = [
            SHELL_COMMAND_ARGUMENT.into(),
            CAPTURE_SCRIPT.into(),
            SHELL_ARGUMENT_ZERO.into(),
            capture.as_os_str().to_owned(),
        ];
        let _owner = waitable_child_spawn_guard().unwrap();
        let deadline = Instant::now() + GATE_WAIT_BUDGET;

        let result = if private {
            spawn_detached_with_private_input(
                Path::new(SHELL_EXECUTABLE),
                &arguments,
                DetachedStandardError::Null,
                PRIVATE_INPUT,
                deadline,
            )
        } else {
            spawn_detached_before_deadline(
                Path::new(SHELL_EXECUTABLE),
                &arguments,
                DetachedStandardError::Null,
                deadline,
            )
        };

        assert!(result.is_err());
        assert!(Instant::now() >= deadline);
        assert!(!capture.exists());
    }
}
