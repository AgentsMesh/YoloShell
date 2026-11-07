use anyhow::Result;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_agent_contract::BINDING_FILE_NAME;

#[path = "constants.rs"]
pub mod constants;
#[path = "construction.rs"]
mod construction;
#[cfg(unix)]
#[path = "directory/directory.rs"]
#[doc(hidden)]
pub mod directory;
#[cfg(unix)]
#[path = "directory/directory_identity.rs"]
mod directory_identity;
#[path = "directory/directory_stream.rs"]
#[doc(hidden)]
pub mod directory_stream;
#[path = "durable/durable_json.rs"]
pub(crate) mod durable_json;
#[path = "durable/durable_json_guarded.rs"]
mod durable_json_guarded;
#[cfg(unix)]
#[path = "durable/durable_json_read.rs"]
mod durable_json_read;
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
#[path = "durable/durable_json_retained.rs"]
mod durable_json_retained;
#[path = "durable/durable_json_serialized.rs"]
mod durable_json_serialized;
#[path = "ensure.rs"]
mod ensure;
#[cfg(unix)]
#[path = "hash.rs"]
mod hash;
#[cfg(unix)]
#[path = "identity.rs"]
#[doc(hidden)]
pub mod identity;
#[cfg(unix)]
#[path = "isolated/isolated_test_socket_root.rs"]
mod isolated_test_socket_root;
#[cfg(unix)]
pub use yolo_paths_core::platform;
#[path = "private/private_bytes.rs"]
mod private_bytes;
#[cfg(unix)]
#[path = "private/private_file.rs"]
#[doc(hidden)]
pub mod private_file;
#[cfg(unix)]
#[path = "private/file/lock/private_file_lock.rs"]
#[doc(hidden)]
pub mod private_file_lock;
#[path = "private/private_parent.rs"]
mod private_parent;
#[cfg(unix)]
#[path = "process/process_file_lock.rs"]
#[doc(hidden)]
pub mod process_file_lock;
#[path = "resume.rs"]
mod resume;
#[cfg(unix)]
#[path = "retained_json_observation.rs"]
mod retained_json_observation;
#[cfg(unix)]
#[path = "socket/socket_activation.rs"]
pub mod socket_activation;
#[cfg(unix)]
#[path = "socket/namespace/socket_namespace.rs"]
pub mod socket_namespace;
#[cfg(unix)]
#[path = "socket/namespace/socket_namespace_path.rs"]
pub(crate) mod socket_path;
#[cfg(unix)]
#[path = "socket/stage/socket_stage.rs"]
pub(crate) mod socket_stage;
#[cfg(feature = "persistence-test-support")]
#[path = "../test_support/persistence_api.rs"]
#[doc(hidden)]
pub mod test_support;
pub use yolo_paths_core::validation;
#[path = "worker_artifacts.rs"]
pub mod worker_artifacts;
pub use crate::ReadVersionChanged;
use constants::*;
pub use constants::{
    MAX_DURABLE_JSON_BYTES, MAX_PRIVATE_DIRECTORY_ENTRIES, PERMISSION_MODE_MASK, PRIVATE_FILE_MODE,
    SOCKET_ROOT_ENVIRONMENT,
};
#[doc(hidden)]
pub use durable_json::write_expected_entry_guarded_with_commit_observer;
pub use durable_json::{atomic_write as atomic_write_json, read as read_json};
#[doc(hidden)]
pub use durable_json_guarded::{atomic_write_entry_guarded, read_entry_guarded};
#[doc(hidden)]
pub(crate) use durable_json_guarded::{
    atomic_write_entry_guarded_with_commit_fence, write_entry_expected_with_commit_fence,
    write_entry_expected_with_commit_fence_with_target_lock_authority,
    write_entry_expected_with_commit_observer,
    write_entry_expected_with_commit_observer_with_target_lock_authority,
};
#[doc(hidden)]
pub use durable_json_guarded::{
    read_entry_guarded_with_identity, read_entry_guarded_with_versions,
};
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
pub(crate) use durable_json_retained::read_retained_entry_guarded_with_opened;
pub(crate) use durable_json_serialized::write_entry_expected_serialized;
#[cfg(unix)]
pub use isolated_test_socket_root::IsolatedTestSocketRoot;
pub use private_bytes::write_private_bytes_guarded;
use private_parent::create_private_dir;
pub use private_parent::validate_home;
#[doc(hidden)]
pub use private_parent::{ensure_private_parent, secure_private_parent};
pub use resume::ResumeMarkerNamespace;
#[cfg(unix)]
#[doc(hidden)]
pub use retained_json_observation::RetainedJsonFileObservation;
#[cfg(unix)]
#[doc(hidden)]
pub use socket_namespace::SocketNamespaceScanner;
#[derive(Debug, Clone)]
pub struct Paths {
    home: PathBuf,
    socket_root_override: Option<PathBuf>,
}

impl Paths {
    pub fn catalog(&self) -> PathBuf {
        self.home.join(CATALOG_FILE_NAME)
    }

    pub fn runtime_dir(&self) -> PathBuf {
        self.home.join(RUNTIME_DIRECTORY)
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.home.join(SESSIONS_DIRECTORY)
    }

    pub fn session_dir(&self, id: Uuid) -> PathBuf {
        self.sessions_dir().join(id.to_string())
    }

    pub fn manifest(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join(MANIFEST_FILE_NAME)
    }

    pub fn runtime_record(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join(RUNTIME_RECORD_FILE_NAME)
    }

    pub fn worker_launch(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join(WORKER_LAUNCH_FILE_NAME)
    }

    pub fn agent_binding(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join(BINDING_FILE_NAME)
    }

    pub fn terminal_dir(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join(TERMINAL_DIRECTORY)
    }

    pub fn journal(&self, id: Uuid) -> PathBuf {
        self.terminal_dir(id).join(JOURNAL_SEGMENT_FILE_NAME)
    }

    pub fn snapshot(&self, id: Uuid) -> PathBuf {
        self.terminal_dir(id).join(SNAPSHOT_FILE_NAME)
    }

    pub fn events_dir(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join(EVENTS_DIRECTORY)
    }

    pub fn logs_dir(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join(LOG_DIRECTORY)
    }

    pub fn exit_record(&self, id: Uuid) -> PathBuf {
        self.events_dir(id).join(EXIT_RECORD_FILE_NAME)
    }

    pub fn terminate_tombstone(&self, id: Uuid) -> PathBuf {
        self.events_dir(id).join(TERMINATE_TOMBSTONE_FILE_NAME)
    }
}

#[cfg(unix)]
pub use private_file::remove_private_file;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod tests;
