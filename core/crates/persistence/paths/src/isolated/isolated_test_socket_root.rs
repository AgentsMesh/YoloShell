use super::constants::*;
use super::directory::PrivateDirectory;
use super::identity::ObjectIdentity;
use anyhow::Result;
use std::ffi::CString;
use std::fs::File;
use std::path::{Path, PathBuf};

#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "isolated_test_socket_root_adopt.rs"]
mod adopt;
#[path = "isolated_test_socket_root_create.rs"]
mod create;
#[path = "isolated_test_socket_root_finalize.rs"]
mod finalize;
#[path = "isolated_test_socket_root_hooks.rs"]
mod hooks;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IsolatedTestSocketRootStage {
    Active,
    Retired,
    Removed,
}

pub struct IsolatedTestSocketRoot {
    parent: File,
    parent_identity: ObjectIdentity,
    root: PrivateDirectory,
    root_identity: ObjectIdentity,
    active_name: CString,
    active_path: PathBuf,
    name: CString,
    path: PathBuf,
    stage: IsolatedTestSocketRootStage,
    removal_synced: bool,
    finalization_attempted: bool,
}

impl IsolatedTestSocketRoot {
    pub fn create() -> Result<Self> {
        create::create()
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn adopt(path: &Path) -> Result<Self> {
        adopt::adopt(path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn finalize(&mut self) -> Result<()> {
        // Explicit callers may retry after resolving a failure. Drop must not
        // allocate a second cleanup budget after that result was reported.
        self.finalization_attempted = true;
        finalize::finalize(self)
    }

    #[cfg(test)]
    pub(crate) fn hold_lifecycle_for_test(
        &self,
    ) -> Result<super::socket_namespace::SocketNamespaceLifecycleLock> {
        super::socket_namespace::SocketNamespaceLifecycleLock::try_acquire(&self.root)?
            .ok_or_else(|| anyhow::anyhow!(finalize::LIFECYCLE_BUSY_ERROR))
    }
}

impl Drop for IsolatedTestSocketRoot {
    fn drop(&mut self) {
        if !self.finalization_attempted {
            let _ = self.finalize();
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/isolated/isolated_test_socket_root_tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/isolated/isolated_test_socket_root_adopt_tests.rs"]
mod adopt_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/isolated/isolated_test_socket_root_drop_tests.rs"]
mod drop_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/isolated/isolated_test_socket_root_fault_tests.rs"]
mod fault_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/isolated/isolated_test_socket_root_security_tests.rs"]
mod security_tests;
