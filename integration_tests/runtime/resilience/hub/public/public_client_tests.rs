use super::support::deadline;

#[path = "public_client/constants.rs"]
mod constants;
#[path = "public_client_credentials.rs"]
mod credentials;
#[path = "public_client_denials.rs"]
#[cfg(target_os = "macos")]
mod denials;
#[path = "public_client_driver.rs"]
mod driver;
#[path = "public_client/test_support/driver_lease_peer.rs"]
mod driver_lease_peer;
#[path = "public_client/integration_tests/driver_lease.rs"]
#[cfg(target_os = "macos")]
mod driver_lease_tests;
#[path = "public_client_fixture.rs"]
mod fixture;
#[path = "public_client_launch.rs"]
mod launch;
#[path = "public_client_multihop.rs"]
#[cfg(target_os = "macos")]
mod multihop;
#[path = "public_multihop_ingress.rs"]
#[cfg(target_os = "macos")]
mod multihop_ingress;
#[path = "public_multihop_tui.rs"]
#[cfg(target_os = "macos")]
mod multihop_tui;
#[path = "public_client_output.rs"]
mod output;
#[path = "public_client_revocation.rs"]
#[cfg(target_os = "macos")]
mod revocation;
#[path = "public_client_shell.rs"]
mod shell;
#[path = "public_client_tui.rs"]
#[cfg(target_os = "macos")]
mod tui;
#[path = "public_client_observer.rs"]
mod observer;
#[path = "public_client_workspace.rs"]
mod workspace;
