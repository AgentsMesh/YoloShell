#[path = "canonical.rs"]
mod canonical;
#[path = "child_spawn_fence.rs"]
mod child_spawn_fence;
#[path = "constants.rs"]
mod constants;
#[path = "lifecycle.rs"]
mod lifecycle;
#[path = "location.rs"]
mod location;
pub use location::existing_lock_path;
#[path = "lock.rs"]
mod lock;
#[path = "mach_lock.rs"]
mod mach_lock;
#[path = "retire.rs"]
mod retire;
#[path = "scavenge.rs"]
mod scavenge;
#[path = "stream.rs"]
mod stream;

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

#[doc(hidden)]
pub use canonical::canonical_logical_path;
pub use child_spawn_fence::CoordinationChildSpawnFence;
use constants::*;
pub use lifecycle::LifecycleLock as LifecycleGuard;
#[doc(hidden)]
pub use lock::CoordinationLock;
pub use mach_lock::CoordinationLease;
#[cfg(not(test))]
pub use scavenge::CoordinationScanner;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoordinationScope {
    HostHome,
    HostEndpoint,
    Worker,
    BrokerInstance,
    BrokerLaunchPublication,
    BrokerReconcile,
    SocketMaintenance,
}

impl CoordinationScope {
    fn label(self) -> &'static str {
        match self {
            Self::HostHome => HOST_HOME_SCOPE,
            Self::HostEndpoint => HOST_ENDPOINT_SCOPE,
            Self::Worker => WORKER_SCOPE,
            Self::BrokerInstance => BROKER_INSTANCE_SCOPE,
            Self::BrokerLaunchPublication => BROKER_LAUNCH_PUBLICATION_SCOPE,
            Self::BrokerReconcile => BROKER_RECONCILE_SCOPE,
            Self::SocketMaintenance => SOCKET_MAINTENANCE_SCOPE,
        }
    }
}

pub fn lock_path(
    scope: CoordinationScope,
    logical_path: &Path,
    discriminator: Option<&str>,
) -> Result<PathBuf> {
    let (root, path) = location::lock_location(scope, logical_path, discriminator)?;
    crate::paths::directory::PrivateDirectory::ensure(&root)?;
    Ok(path)
}

#[cfg(test)]
pub(crate) fn catalog_alias_directory(
    catalog_namespace: &Path,
) -> Result<crate::paths::directory::PrivateDirectory> {
    let coordination_root = private_root()?;
    let root = catalog_alias_root(&coordination_root)?;
    let digest = catalog_alias_shard_name(catalog_namespace)?;
    root.ensure_descendant(&root.path().join(digest))
}

pub fn catalog_alias_shard_name(catalog_namespace: &Path) -> Result<String> {
    crate::paths::validation::validate_absolute(catalog_namespace)?;
    let canonical = canonical::canonical_logical_path(catalog_namespace)?;
    Ok(catalog_alias_digest(&canonical))
}

pub fn catalog_alias_root(
    root: &crate::paths::directory::PrivateDirectory,
) -> Result<crate::paths::directory::PrivateDirectory> {
    root.ensure_descendant(&root.path().join(CATALOG_ALIAS_DIRECTORY))
}

pub fn maintenance_owner_path() -> PathBuf {
    coordination_root().join(MAINTENANCE_OWNER_LOCK_FILE)
}

pub fn private_root() -> Result<crate::paths::directory::PrivateDirectory> {
    crate::paths::directory::PrivateDirectory::ensure(&coordination_root())
}

pub fn try_lifecycle(
    root: &crate::paths::directory::PrivateDirectory,
) -> Result<Option<LifecycleGuard>> {
    LifecycleGuard::try_acquire(root)
}

fn catalog_alias_digest(canonical_namespace: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CATALOG_ALIAS_DIGEST_DOMAIN);
    hasher.update([DIGEST_SEPARATOR]);
    hasher.update(canonical_namespace.as_os_str().as_bytes());
    let mut encoded = String::with_capacity(LOCK_DIGEST_HEX_BYTES);
    for byte in hasher.finalize() {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

pub fn is_catalog_alias_digest(name: &std::ffi::OsStr) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == CATALOG_ALIAS_DIGEST_HEX_BYTES
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

#[cfg(not(test))]
fn coordination_root() -> PathBuf {
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    let effective_user = unsafe { libc::geteuid() };
    PathBuf::from(format!("{COORDINATION_ROOT_PREFIX}{effective_user}"))
}

#[cfg(test)]
fn coordination_root() -> PathBuf {
    use std::sync::OnceLock;
    static ROOT: OnceLock<tempfile::TempDir> = OnceLock::new();
    ROOT.get_or_init(|| {
        tempfile::Builder::new()
            .prefix(TEST_COORDINATION_ROOT_PREFIX)
            .tempdir_in(TEST_COORDINATION_PARENT)
            .expect("create isolated coordination test root")
    })
    .path()
    .to_path_buf()
}

fn lock_digest(
    scope: CoordinationScope,
    logical_path: &Path,
    discriminator: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(scope.label());
    hasher.update([DIGEST_SEPARATOR]);
    hasher.update(logical_path.as_os_str().as_bytes());
    hasher.update([DIGEST_SEPARATOR]);
    hasher.update(discriminator.unwrap_or_default());
    let mut encoded = String::with_capacity(LOCK_DIGEST_HEX_BYTES);
    for byte in hasher.finalize() {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/coordination/gc_adversarial_tests.rs"]
mod gc_adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/coordination/gc_tests.rs"]
mod gc_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/coordination/scanner_adversarial_tests.rs"]
mod scanner_adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/coordination/tests.rs"]
mod tests;
