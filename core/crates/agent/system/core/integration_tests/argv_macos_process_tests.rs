use super::process_argv_bounded;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const ENTRY_NAME: &str = yolo_hooks_runtime::provider_names::CODEX;
const SHELL: &str = "/bin/zsh";
const SLEEP: &str = "/bin/sleep";
const SLEEP_SECONDS: &str = "30";
const SHELL_ARGUMENT_ZERO: &str = "yolo-argv-test";
const SPAWN_SCRIPT: &str = "exec codex \"$1\"";
const EXEC_TRANSITION_TIMEOUT: Duration = Duration::from_secs(2);
const OBSERVATION_RETRY_INTERVAL: Duration = Duration::from_millis(10);

#[test]
fn zsh_path_launch_preserves_bare_argv_zero_but_exposes_the_resolved_image() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let entry = root.join(ENTRY_NAME);
    std::os::unix::fs::symlink(SLEEP, &entry).unwrap();
    let image = std::fs::canonicalize(SLEEP).unwrap();
    let mut command = Command::new(SHELL);
    command
        .args(["-f", "-c", SPAWN_SCRIPT, SHELL_ARGUMENT_ZERO, SLEEP_SECONDS])
        .env_clear()
        .env(crate::constants::PATH_ENV, &root)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let _spawn_guard = yolo_process_spawn::waitable_child_spawn_guard().unwrap();
    let shell = command.spawn().unwrap();
    let pid = shell.id();
    let _process = ProcessGuard::new(shell);

    wait_for_exec(pid, &image);
}

fn wait_for_exec(pid: u32, entry: &std::path::Path) {
    let deadline = Instant::now() + EXEC_TRANSITION_TIMEOUT;
    loop {
        let executable = yolo_process::macos_native::executable_path(pid);
        let snapshot =
            process_argv_bounded(pid, crate::constants::MAX_SINGLE_PROCESS_ARGV_BYTES, &[]);
        if executable.is_ok_and(|value| value == entry)
            && snapshot
                .as_ref()
                .is_ok_and(|value| value.values == [ENTRY_NAME, SLEEP_SECONDS])
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "zsh child did not exec PATH entry"
        );
        std::thread::sleep(OBSERVATION_RETRY_INTERVAL);
    }
}

struct ProcessGuard {
    shell: Child,
}

impl ProcessGuard {
    fn new(shell: Child) -> Self {
        Self { shell }
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.shell.kill();
        let _ = self.shell.wait();
    }
}
