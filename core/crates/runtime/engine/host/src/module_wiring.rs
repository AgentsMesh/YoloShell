mod agent_binding;
mod api;
#[path = "archived_history.rs"]
mod archived_history;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/unit/automatic_resume_test_probes.rs"]
mod automatic_resume_test_probes;
mod automatic_worker_crash_exit;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/unit/automatic_worker_crash_exit_test_hook.rs"]
mod automatic_worker_crash_exit_test_hook;
mod build;
mod catalog;
mod completed_attachment_constants;
mod config;
mod constants;
mod creation;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/unit/creation/creation_checkpoint_test_hook.rs"]
mod creation_checkpoint_test_hook;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/unit/creation/creation_receipt_staging_support_tests.rs"]
mod creation_receipt_staging_support_tests;
#[cfg(feature = "failed-rearm-test-hooks")]
#[doc(hidden)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/integration/failed_rearm_process_fixture.rs"]
pub mod failed_rearm_process_fixture;
mod idle_shutdown;
mod ingress_constants;
mod instance;
mod lifecycle;
#[cfg(target_os = "linux")]
mod linux_retirement;
mod reconcile;
mod socket;
#[cfg(any(test, feature = "reconcile-test-hooks"))]
#[doc(hidden)]
pub use reconcile::reconcile_test_hook;
mod agent_watch;
mod request_drain;
mod resume;
mod secondary_custody;
#[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
#[doc(hidden)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/unit/secondary_custody/secondary_custody_release_test_hook.rs"]
pub mod secondary_custody_release_test_hook;
mod session;
mod terminal_containment;
mod worker;

#[path = "client_connection.rs"]
mod client_connection;
#[path = "completed_attachment_view.rs"]
mod completed_attachment_view;
#[path = "connection_io.rs"]
mod connection_io;
#[path = "dispatch.rs"]
mod dispatch;
#[cfg(target_os = "linux")]
#[path = "linux_containment_fallback.rs"]
mod linux_containment_fallback;
#[cfg(target_os = "linux")]
#[path = "linux_containment_recovery.rs"]
mod linux_containment_recovery;
#[path = "recovery_safety.rs"]
mod recovery_safety;
#[path = "recovery_safety_evidence.rs"]
mod recovery_safety_evidence;
#[path = "recovery_write_compensation.rs"]
mod recovery_write_compensation;
#[path = "request_io.rs"]
mod request_io;
#[path = "runtime_bootstrap.rs"]
mod runtime_bootstrap;
#[path = "runtime_view_recovery.rs"]
mod runtime_view_recovery;
#[path = "secondary_custody_rpc.rs"]
mod secondary_custody_rpc;
#[path = "server_shutdown.rs"]
mod server_shutdown;
#[path = "unreachable_runtime_termination.rs"]
mod unreachable_runtime_termination;
#[path = "view_helpers.rs"]
mod view_helpers;
#[path = "watchdog.rs"]
mod watchdog;
#[cfg(test)]
include!("../test_support/shared.rs");

pub(crate) use completed_attachment_view::*;
pub(crate) use connection_io::*;
pub(crate) use dispatch::*;
#[cfg(target_os = "linux")]
pub(crate) use linux_containment_fallback::*;
#[cfg(target_os = "linux")]
pub(crate) use linux_containment_recovery::*;
pub(crate) use recovery_safety::*;
pub(crate) use recovery_safety_evidence::*;
pub(crate) use recovery_write_compensation::*;
pub(crate) use request_io::*;
pub(crate) use runtime_bootstrap::*;
pub(crate) use runtime_observation::*;
pub(crate) use runtime_view_recovery::*;
pub(crate) use secondary_custody_rpc::*;
pub(crate) use server_shutdown::*;
pub(crate) use unreachable_runtime_termination::*;
pub(crate) use view_helpers::*;
pub(crate) use watchdog::*;
pub(crate) use yolo_host_runtime_observation as runtime_observation;
pub(crate) use yolo_host_runtime_views::*;
pub(crate) use yolo_host_shutdown_signal::HostShutdownSignals;
