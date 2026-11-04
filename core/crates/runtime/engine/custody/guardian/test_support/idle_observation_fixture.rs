use super::*;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub(super) const CONTENT: &[u8] = b"{}";
pub(super) const REPLACEMENT_EXTENSION: &str = "replacement";
pub(super) const UNRELATED_NAME: &str = "unrelated";
pub(super) const RUNTIME_NAME: &str = "runtime.json";
pub(super) const CHMOD: &str = "/bin/chmod";
pub(super) const ADD_ACL: &str = "+a";
pub(super) const TEST_ACL: &str = "everyone allow read";
pub(super) const ALTERED_MODE: u32 = 0o750;
const INITIAL_MODE: u32 = 0o700;
const PARENT_NAME: &str = "ancestor";
const RECORD_NAME: &str = "custody.json";
const SHELL: &str = "/bin/sh";
const SHELL_COMMAND: &str = "-c";
const WAIT_FOR_INPUT: &str = "printf 'ready\\n'; read value";
const CHILD_READY: &str = "ready\n";

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) parent: PathBuf,
    pub(super) file: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join(PARENT_NAME);
        std::fs::create_dir(&parent).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(INITIAL_MODE)).unwrap();
        let file = parent.join(RECORD_NAME);
        std::fs::write(&file, CONTENT).unwrap();
        Self {
            _root: root,
            parent,
            file,
        }
    }

    pub(super) fn watch(&self) -> KernelObservation {
        KernelObservation::watch(&[self.file.clone()], Vec::new()).unwrap()
    }
}

pub(super) struct Child(pub(super) std::process::Child);

impl Child {
    pub(super) fn start() -> Self {
        let mut command = Command::new(SHELL);
        command
            .args([SHELL_COMMAND, WAIT_FOR_INPUT])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = Self(crate::process::spawn_waitable_test_child(command));
        let mut line = String::new();
        BufReader::new(child.0.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert_eq!(line, CHILD_READY);
        child
    }

    pub(super) fn stop(&mut self) {
        self.0.stdin.take();
        self.0.wait().unwrap();
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
