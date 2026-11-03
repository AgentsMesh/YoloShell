use super::constants::*;
use super::directory::PrivateDirectory;
use super::{Paths, directory_stream as stream, hash, socket_path, socket_stage, validate_home};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[path = "socket_namespace_active_endpoints.rs"]
mod active_endpoints;
#[path = "socket_namespace_admission.rs"]
mod admission;
#[path = "socket_namespace_authority.rs"]
mod authority;
#[path = "socket_namespace_candidate.rs"]
mod candidate;
#[path = "socket_namespace_cleanup.rs"]
mod cleanup;
#[path = "socket_namespace_directory_certificate.rs"]
mod directory_certificate;
#[path = "socket_namespace_directory_lease.rs"]
mod directory_lease;
#[path = "socket_namespace_empty_remove.rs"]
mod empty_remove;
#[path = "socket_namespace_endpoints.rs"]
mod endpoints;
#[path = "socket_namespace_lock.rs"]
mod lifecycle_lock;
#[path = "socket_namespace_location.rs"]
mod location;
#[path = "socket_namespace_paths.rs"]
mod paths;
#[path = "socket_namespace_registered_cleanup.rs"]
mod registered_cleanup;
#[path = "socket_namespace_registration.rs"]
mod registration;
#[path = "socket_namespace_release.rs"]
mod release;
#[path = "socket_namespace_root_mutation.rs"]
mod root_mutation;
#[path = "socket_namespace_root_scan.rs"]
mod root_scan;
#[path = "socket_namespace_root_selection.rs"]
mod root_selection;
#[path = "socket_namespace_root_visit.rs"]
mod root_visit;
#[path = "socket_namespace_scanner.rs"]
mod scanner;
#[path = "socket_namespace_stage_scan.rs"]
mod stage_scan;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../test_support/integration/socket_namespace_test_construction.rs"]
mod test_construction;
#[path = "socket_namespace_unbound.rs"]
mod unbound;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NamespaceBinding {
    schema_version: u32,
    effective_user_id: u32,
    home_digest: String,
}

#[derive(Clone, Debug)]
struct SocketNamespaceLocation {
    root: PathBuf,
    directory: PathBuf,
    binding: NamespaceBinding,
    overridden_root: bool,
}

#[derive(Debug)]
pub struct SocketNamespace {
    location: SocketNamespaceLocation,
    root: PrivateDirectory,
    directory: PrivateDirectory,
    authority: authority::Authority,
    preserve_directory_on_release: bool,
}

pub(crate) use lifecycle_lock::SocketNamespaceLifecycleLock;
pub use release::NamespaceRelease;
pub use scanner::SocketNamespaceScanner;

impl SocketNamespace {
    /// Release descriptors and binding custody without moving an unproven directory.
    /// Call after an endpoint owner discovers replacement or failed exact cleanup.
    pub fn preserve_on_release(&mut self) {
        self.preserve_directory_on_release = true;
    }

    pub fn directory(&self) -> Result<PrivateDirectory> {
        self.directory.try_clone()
    }

    pub fn root_path(&self) -> &Path {
        &self.location.root
    }

    pub fn scanner(&self) -> Result<SocketNamespaceScanner> {
        SocketNamespaceScanner::from_root(&self.root, self.location.overridden_root)
    }

    pub fn verify(&self) -> Result<()> {
        self.root
            .verify_path()
            .context("verify socket namespace root")?;
        self.directory
            .verify_path()
            .context("verify socket namespace directory")?;
        self.authority
            .verify(&self.directory, &self.location.binding)
    }
}

fn default_private_root(effective_user_id: u32) -> PathBuf {
    PathBuf::from(format!("{SOCKET_ROOT_PREFIX}{effective_user_id:x}"))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_test_suite.rs"]
mod test_suite;
