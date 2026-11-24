#[path = "boot_identity.rs"]
mod boot_identity;
pub mod constants;
#[cfg(any(test, feature = "identity-test-support"))]
#[path = "../test_support/identity_relation.rs"]
mod identity_relation_test_support;
#[cfg(target_os = "linux")]
pub mod linux_native;
#[cfg(target_os = "macos")]
#[doc(hidden)]
pub mod macos_kernel;
#[cfg(target_os = "macos")]
#[doc(hidden)]
pub mod macos_lineage;
#[path = "process.rs"]
mod process;
#[cfg(not(target_os = "macos"))]
#[doc(hidden)]
pub mod procfs;
#[cfg(test)]
#[path = "../test_support/mod.rs"]
mod test_support;

pub use boot_identity::{
    BootIdentity, BootIdentityRelation, MAX_BOOT_IDENTITY_BYTES, boot_id, boot_identity_relation,
};
#[cfg(any(test, feature = "identity-test-support"))]
#[doc(hidden)]
pub use identity_relation_test_support::{
    ProcessIdentityOverride, install_test_process_identity_relation,
};
#[cfg(target_os = "linux")]
pub use process::process_identity_is_inactive;
pub use process::{
    ProcessIdentityObservation, ProcessIdentityRelation, is_process_alive,
    observe_process_identity, process_identity_matches, process_identity_relation,
    process_start_time, try_process_identity_matches,
};
