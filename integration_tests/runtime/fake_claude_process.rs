use crate::claude_scenario::{
    ClaudeScenario, HOOK_SOCKET_ENV, HookAvailability, SCENARIO_FILE_ENV,
};
use crate::claude_wire::ClaudeHookFrame;
use std::fs::File;
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

#[path = "fake_claude_process_custody.rs"]
mod custody;
use custody::ChildCustody;
#[path = "fake_claude_process_stream.rs"]
mod stream_reader;
use stream_reader::read_frames;

const FAKE_BINARY_ENV: &str = "YOLO_TEST_CLAUDE_BINARY";
const SCENARIO_FILE_NAME: &str = "scenario.json";
const SOCKET_FILE_NAME: &str = "hook.sock";
const DISPLAY_ENV: &str = "DISPLAY";
const WAYLAND_DISPLAY_ENV: &str = "WAYLAND_DISPLAY";
const SSH_ASKPASS_ENV: &str = "SSH_ASKPASS";
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(5);
const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const ACCEPT_TIMEOUT_ERROR: &str = "fake Claude did not connect to its hook socket";
const CHILD_TIMEOUT_ERROR: &str = "fake Claude did not exit after scenario delivery";
const BINARY_UNAVAILABLE: &str = "fake Claude binary is unavailable";

pub(crate) struct ProcessResult {
    pub(crate) status: ExitStatus,
    pub(crate) frames: Vec<ClaudeHookFrame>,
    pub(crate) stderr: String,
}

pub(crate) fn run(scenario: &ClaudeScenario, arguments: &[&str]) -> ProcessResult {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory
        .path()
        .join(yolo_runtime::agent::claude::constants::EXECUTABLE);
    std::os::unix::fs::symlink(fake_binary(), &executable).unwrap();
    let scenario_path = directory.path().join(SCENARIO_FILE_NAME);
    serde_json::to_writer(File::create(&scenario_path).unwrap(), scenario).unwrap();
    let socket_path = directory.path().join(SOCKET_FILE_NAME);
    let listener = UnixListener::bind(&socket_path).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut child = ChildCustody::new(
        command(&executable, &scenario_path, &socket_path, arguments)
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + PROCESS_EXIT_TIMEOUT;
    let frames = if scenario.hook_availability == HookAvailability::Enabled {
        accept(&listener, child.child_mut(), deadline)
            .unwrap()
            .map(|stream| read_frames(stream, deadline).unwrap())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    wait_for_exit(child.child_mut(), deadline).unwrap();
    let output = child.finish().unwrap();
    ProcessResult {
        status: output.status,
        frames,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn command(executable: &Path, scenario: &Path, socket: &Path, arguments: &[&str]) -> Command {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .env(SCENARIO_FILE_ENV, scenario)
        .env(HOOK_SOCKET_ENV, socket)
        .env_remove(DISPLAY_ENV)
        .env_remove(WAYLAND_DISPLAY_ENV)
        .env_remove(SSH_ASKPASS_ENV)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

fn accept(
    listener: &UnixListener,
    child: &mut Child,
    deadline: Instant,
) -> io::Result<Option<UnixStream>> {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false)?;
                return Ok(Some(stream));
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error),
        }
        if child.try_wait()?.is_some() {
            return Ok(None);
        }
        if Instant::now() >= deadline {
            return Err(io::Error::other(ACCEPT_TIMEOUT_ERROR));
        }
        std::thread::sleep(ACCEPT_POLL_INTERVAL);
    }
}

fn wait_for_exit(child: &mut Child, deadline: Instant) -> Result<(), &'static str> {
    loop {
        if child.try_wait().map_err(|_| CHILD_TIMEOUT_ERROR)?.is_some() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(CHILD_TIMEOUT_ERROR);
        }
        std::thread::sleep(ACCEPT_POLL_INTERVAL);
    }
}

#[cfg(test)]
pub(crate) fn collect_frames_for_test(
    stream: UnixStream,
    child: Child,
    deadline: Instant,
) -> io::Result<Vec<ClaudeHookFrame>> {
    let _custody = ChildCustody::new(child);
    read_frames(stream, deadline)
}

fn fake_binary() -> PathBuf {
    let configured = std::env::var_os(FAKE_BINARY_ENV).expect(BINARY_UNAVAILABLE);
    std::fs::canonicalize(configured).expect(BINARY_UNAVAILABLE)
}
