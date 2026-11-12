use crate::container::engine::SystemEngine;
use anyhow::{Result, ensure};
use std::cell::Cell;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_release_contracts::{DOCKER_CLIENT_SHA256, DOCKER_CLIENT_VERSION};
use yolo_release_filesystem::{DirectoryHandle, TemporaryDirectory};
use yolo_release_gate_system::GateSnapshot;
use yolo_release_tool_custody::DockerClientTool;

pub const CONTAINER_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const OTHER_ID: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
pub const CONTAINER_NAME: &str = "owned-release-container";
pub const ENGINE_ID: &str = "owned-release-engine";
pub const CLIENT_CHANGED: &str = "fixture pinned client changed";
pub const DAEMON_FAILURE: &str = "fixture daemon unavailable";
const PREFIX: &str = "yolo-system-engine-test-";
const EXECUTABLE: &str = "docker";
const EXECUTABLE_MODE: u32 = 0o700;
const CALLS: &str = "calls";
const SCRIPT: &str = r#"#!/bin/bash
set -eu
root=${0%/*}
printf '%s\n' "$*" >> "$root/calls"
if [ -f "$root/reject" ]; then
  /bin/cat "$root/reject" >&2
  exit 71
fi
case "$1 $2" in
  'info --format')
    [ "$#" -eq 3 ] && [ "$3" = '{{.ID}}' ]
    /bin/cat "$root/identity" ;;
  'container ls')
    [ "$#" -eq 7 ] && [ "$3 $4 $5 $6" = '--all --no-trunc --quiet --filter' ]
    [ "$7" = "$(/bin/cat "$root/filter")" ]
    /bin/cat "$root/list" ;;
  'container inspect')
    [ "$#" -eq 3 ] && [ "$3" = "$(/bin/cat "$root/id")" ]
    /bin/cat "$root/inspect" ;;
  'container rm')
    [ "$#" -eq 4 ] && [ "$3" = '--force' ]
    [ "$4" = "$(/bin/cat "$root/id")" ]
    /bin/cat "$root/remove"
    if [ -f "$root/settle-after-remove" ]; then
      : > "$root/list"
      printf 'id=%s' "$4" > "$root/filter"
    fi ;;
  *) exit 72 ;;
esac
"#;

pub struct DockerFixture {
    pub root: TemporaryDirectory,
    executable: PathBuf,
    pub checks: Cell<usize>,
    pub fail_check: Cell<Option<usize>>,
}
impl DockerFixture {
    pub fn new() -> Self {
        let root = TemporaryDirectory::create(PREFIX).unwrap();
        let executable = root.path().join(EXECUTABLE);
        fs::write(&executable, SCRIPT).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
        let fixture = Self {
            root,
            executable,
            checks: Cell::new(0),
            fail_check: Cell::new(None),
        };
        fixture.write("identity", ENGINE_ID.as_bytes());
        fixture.write("id", CONTAINER_ID.as_bytes());
        fixture.write("filter", format!("name=^/{CONTAINER_NAME}$").as_bytes());
        fixture.write("list", CONTAINER_ID.as_bytes());
        fixture.write("inspect", &inspection(CONTAINER_ID));
        fixture.write("remove", b"");
        fixture
    }
    pub fn write(&self, name: &str, bytes: &[u8]) {
        fs::write(self.root.path().join(name), bytes).unwrap();
    }
    pub fn calls(&self) -> Vec<String> {
        match fs::read_to_string(self.root.path().join(CALLS)) {
            Ok(calls) => calls.lines().map(str::to_owned).collect(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => panic!("read Docker command evidence: {error}"),
        }
    }
    pub fn engine<'a>(&'a self, snapshot: &'a GateSnapshot) -> SystemEngine<'a> {
        SystemEngine {
            snapshot,
            docker: self,
        }
    }
}
impl DockerClientTool for DockerFixture {
    fn path(&self) -> &Path {
        &self.executable
    }
    fn resource_directory(&self) -> Option<&DirectoryHandle> {
        Some(self.root.handle())
    }
    fn canonical_path(&self) -> &Path {
        &self.executable
    }
    fn sha256(&self) -> &str {
        DOCKER_CLIENT_SHA256
    }
    fn version(&self) -> &str {
        DOCKER_CLIENT_VERSION
    }
    fn ensure_unchanged(&self) -> Result<()> {
        let check = self.checks.get() + 1;
        self.checks.set(check);
        ensure!(self.fail_check.get() != Some(check), CLIENT_CHANGED);
        Ok(())
    }
}
pub fn inspection(id: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!([{
        "Id": id, "Name": format!("/{CONTAINER_NAME}"),
        "Config": {"Image": "fixture-image", "Labels": {}, "Cmd": []}, "Mounts": []
    }]))
    .unwrap()
}
