use super::*;
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;

pub(super) const HOST_ID: &str = "build-host";
pub(super) const OTHER_HOST_ID: &str = "other-host";
pub(super) const PROFILE_SHA256: &str =
    "37f83877e6529ee438080ff6dd8c126732a9d37dd9043eadc667feeb5791aaed";
pub(super) const OTHER_PROFILE_SHA256: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(super) const PRIVATE_MODE: u32 = 0o600;
pub(super) const UNSAFE_MODE: u32 = 0o644;
pub(super) const LOCK_MARKER: &[u8] = b"Yolo remote host authority v1\n";

pub(super) struct Fixture {
    pub(super) _root: tempfile::TempDir,
    pub(super) document: PathBuf,
    pub(super) lock: PathBuf,
}

impl Fixture {
    pub(super) fn new(document: Value) -> Self {
        let root = tempfile::tempdir().unwrap();
        let document_path = root
            .path()
            .join(format!("{HOST_ID}{AUTHORITY_DOCUMENT_SUFFIX}"));
        let lock = root
            .path()
            .join(format!("{HOST_ID}{AUTHORITY_LOCK_SUFFIX}"));
        write_private(&document_path, &serde_json::to_vec(&document).unwrap());
        write_private(&lock, LOCK_MARKER);
        Self {
            _root: root,
            document: document_path,
            lock,
        }
    }
}

pub(super) fn committed() -> Value {
    json!({
        "version": 1,
        "hostID": HOST_ID,
        "phase": "committed",
        "profile": full_profile(),
        "profileSHA256": PROFILE_SHA256,
    })
}

pub(super) fn full_profile() -> Value {
    json!({
        "hostID": HOST_ID,
        "displayName": "Build host",
        "endpoint": { "user": "builder", "host": "build.example.com", "port": 22 },
        "knownHostsURL": "file:///tmp/yolo-known-hosts",
        "identityURLs": ["file:///tmp/yolo-identity"],
        "useSSHAgent": true,
        "runtime": { "executablePath": "/opt/yolo/yolo", "homePath": "/var/lib/yolo" },
        "access": { "allowedPrograms": ["/usr/bin/git"], "readRoots": ["/srv/work"] },
    })
}

pub(super) fn blocked(mutation_id: Uuid) -> Value {
    json!({
        "version": 1,
        "hostID": HOST_ID,
        "phase": "blocked",
        "mutationID": mutation_id,
    })
}

pub(super) fn write_private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_MODE)).unwrap();
}
