use super::constants::*;
use crate::remote::constants::{
    RUNTIME_EXECUTABLE_MODE, RUNTIME_LINK_PREFIX, RUNTIME_UPLOAD_MODE, RUNTIME_UPLOAD_PREFIX,
};
use russh_sftp::protocol::{FileAttributes, OpenFlags, StatusCode};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[path = "memory_fs/bundle_control.rs"]
mod bundle_control;
#[path = "memory_fs/channel_open.rs"]
mod channel_open;
#[path = "memory_fs/commands.rs"]
mod commands;
#[path = "memory_fs/failures.rs"]
mod failures;
#[path = "memory_fs/hub_peers.rs"]
mod hub_peers;
#[path = "memory_fs/lifecycle.rs"]
mod lifecycle;
#[path = "memory_fs/objects.rs"]
mod objects;
#[path = "memory_fs/operations.rs"]
mod operations;
#[path = "memory_fs/permissions.rs"]
mod permissions;

pub use failures::FailurePoint;
pub use hub_peers::HubPeerLifecycle;

#[derive(Clone)]
pub struct MemoryFilesystem(Arc<Mutex<State>>);

#[derive(Clone)]
pub(super) struct MemoryFile(Arc<Mutex<Entry>>);

struct Entry {
    data: Vec<u8>,
    permissions: u32,
    uid: u32,
}

struct State {
    files: HashMap<String, MemoryFile>,
    directories: HashMap<String, u32>,
    links: HashMap<String, String>,
    failure: Option<FailurePoint>,
    writes: usize,
    activations: usize,
    close_attempts: usize,
    live_handles: usize,
    sftp_sessions_started: usize,
    sftp_sessions_dropped: usize,
    handles_reclaimed_on_drop: usize,
    reported_version: String,
    fsync_supported: bool,
    posix_rename_supported: bool,
    commands: CommandState,
    channel_open: Arc<channel_open::ChannelOpenControl>,
    hub_peer_target: Option<std::net::SocketAddr>,
    hub_peer_lifecycles: HashMap<russh::ChannelId, HubPeerLifecycle>,
    uid_response: (Vec<u8>, Vec<u8>, Option<u32>),
    uid_probes: usize,
}

#[derive(Default)]
struct CommandState {
    live: usize,
    signals: usize,
    closes: usize,
    output_started: usize,
}

impl Default for MemoryFilesystem {
    fn default() -> Self {
        let filesystem = Self(Arc::new(Mutex::new(State {
            files: HashMap::new(),
            directories: HashMap::new(),
            links: HashMap::new(),
            failure: None,
            writes: INITIAL_COUNTER,
            activations: INITIAL_COUNTER,
            close_attempts: NO_CLOSE_ATTEMPTS,
            live_handles: NO_LIVE_HANDLES,
            sftp_sessions_started: INITIAL_COUNTER,
            sftp_sessions_dropped: INITIAL_COUNTER,
            handles_reclaimed_on_drop: NO_DROP_RECLAIMED_HANDLES,
            reported_version: String::new(),
            fsync_supported: true,
            posix_rename_supported: true,
            commands: CommandState::default(),
            channel_open: Arc::new(channel_open::ChannelOpenControl::default()),
            hub_peer_target: None,
            hub_peer_lifecycles: HashMap::new(),
            uid_response: (format!("{MEMORY_UID}\n").into_bytes(), Vec::new(), Some(0)),
            uid_probes: usize::default(),
        })));
        filesystem.insert_file(MEMORY_FILE, FILE_CONTENT, RUNTIME_UPLOAD_MODE);
        filesystem.insert_file(MEMORY_OUTSIDE, OUTSIDE_FILE_CONTENT, RUNTIME_UPLOAD_MODE);
        filesystem.insert_link(MEMORY_LINK, MEMORY_OUTSIDE);
        filesystem
    }
}

impl MemoryFilesystem {
    pub fn set_hub_peer_target(&self, target: std::net::SocketAddr) {
        self.0.lock().unwrap().hub_peer_target = Some(target);
    }
    pub(crate) fn hub_peer_target(&self) -> Option<std::net::SocketAddr> {
        self.0.lock().unwrap().hub_peer_target
    }
    pub fn fail_once(&self, failure: FailurePoint) {
        self.0.lock().unwrap().failure = Some(failure);
    }
    pub fn failure_is_armed(&self, failure: FailurePoint) -> bool {
        self.0.lock().unwrap().failure == Some(failure)
    }
    pub fn set_version(&self, version: &str) {
        self.0.lock().unwrap().reported_version = version.into();
    }
    pub fn version(&self) -> String {
        self.0.lock().unwrap().reported_version.clone()
    }
    pub fn writes(&self) -> usize {
        self.0.lock().unwrap().writes
    }
    pub fn activations(&self) -> usize {
        self.0.lock().unwrap().activations
    }
    pub fn target(&self, path: &str) -> Option<String> {
        self.0.lock().unwrap().links.get(path).cloned()
    }
    pub fn supports_posix_rename(&self) -> bool {
        self.0.lock().unwrap().posix_rename_supported
    }
    pub fn supports_fsync(&self) -> bool {
        self.0.lock().unwrap().fsync_supported
    }
    pub fn set_fsync_supported(&self, supported: bool) {
        self.0.lock().unwrap().fsync_supported = supported;
    }
    pub fn set_posix_rename_supported(&self, supported: bool) {
        self.0.lock().unwrap().posix_rename_supported = supported;
    }
    pub fn insert_directory(&self, path: &str, permissions: u32) {
        self.0
            .lock()
            .unwrap()
            .directories
            .insert(path.into(), permissions);
    }
    pub fn insert_link(&self, link: &str, target: &str) {
        self.0
            .lock()
            .unwrap()
            .links
            .insert(link.into(), target.into());
    }
    pub fn temporary_entries(&self) -> usize {
        let state = self.0.lock().unwrap();
        state
            .files
            .keys()
            .chain(state.links.keys())
            .filter(|path| {
                path.contains(RUNTIME_UPLOAD_PREFIX) || path.contains(RUNTIME_LINK_PREFIX)
            })
            .count()
    }
}
