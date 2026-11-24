#[cfg(all(feature = "adversarial-test-hooks", not(test)))]
#[path = "../test_support/waitable_child/waitable_child_spawn.rs"]
mod waitable_child_spawn;
#[cfg(all(feature = "adversarial-test-hooks", not(test)))]
#[doc(hidden)]
pub use waitable_child_spawn::spawn_waitable_test_child;
#[cfg(all(feature = "adversarial-test-hooks", not(test)))]
#[path = "../test_support/identity_relation_test_hook.rs"]
mod adversarial_identity_relation_test_hook;
#[cfg(all(feature = "adversarial-test-hooks", not(test)))]
#[doc(hidden)]
pub use adversarial_identity_relation_test_hook::{
    Guard as ProcessIdentityRelationGuard, install as install_test_process_identity_relation,
};
#[cfg(all(feature = "adversarial-test-hooks", not(test)))]
#[doc(hidden)]
pub use live_identity::install_test_live_signal_failure;
#[cfg(all(feature = "adversarial-test-hooks", not(test)))]
#[doc(hidden)]
pub use live_identity::install_test_live_signal_observer;
#[cfg(all(feature = "adversarial-test-hooks", not(test)))]
#[doc(hidden)]
pub fn unknown_boot_id_for_test() -> String {
    format!(
        "{}{}",
        crate::constants::UNKNOWN_BOOT_ID_PREFIX,
        std::process::id()
    )
}
#[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
#[path = "../test_support/prepared_retirement_custody.rs"]
mod prepared_retirement_custody;
#[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
#[doc(hidden)]
pub use prepared_retirement_custody::{
    PreparedRetirementCustodyFixture, prepared_retirement_custody_fixture,
};
#[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
#[path = "../test_support/active_secondary_custody.rs"]
mod active_secondary_custody;
#[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
#[doc(hidden)]
pub use active_secondary_custody::{
    ActiveMacosSecondaryCustodyFixture, MacosSecondaryCustodyFixtureAliases,
};
#[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
#[path = "../test_support/retained/retained_pty_probe.rs"]
mod retained_pty_probe;
#[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
#[doc(hidden)]
pub use retained_pty_probe::RetainedPtyProbeFixture;

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub use captured_targets::{install_test_signal_failure, install_test_signal_noop};
#[cfg(all(
    target_os = "macos",
    any(
        all(test, feature = "yolo-integration-tests"),
        feature = "adversarial-test-hooks"
    )
))]
pub use pty_session_proof::install_namespace_loss_drain_pending_once;
#[cfg(all(
    target_os = "macos",
    any(
        all(test, feature = "yolo-integration-tests"),
        feature = "adversarial-test-hooks"
    )
))]
pub use pty_session_proof::install_pre_finalize_hook as install_namespace_loss_pre_finalize_hook;
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
pub use pty_session_proof::process_holds_capability_for_test;
