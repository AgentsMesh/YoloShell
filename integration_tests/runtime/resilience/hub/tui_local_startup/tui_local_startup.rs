#[path = "tui_local_startup_cancellation.rs"]
mod cancellation;
#[path = "tui_local_startup/constants.rs"]
mod constants;
#[path = "tui_local_startup_driver.rs"]
mod driver;
#[path = "tui_local_startup_fixture.rs"]
mod fixture;
#[path = "tui_local_startup_shell.rs"]
mod shell;
use super::local_startup_support::Services;
use constants::*;
use fixture::Installation;

#[tokio::test(flavor = "multi_thread")]
async fn mac_tui_explicit_local_profile_bootstraps_hub_and_reopens_the_same_live_shell() {
    let host = crate::Harness::start().await;
    let original = host.create_cat(SHELL_NAME).await;
    let installation = Installation::new(Some(host.home.path()));
    let terminal = crate::Harness::start().await;
    driver::exercise(&terminal, &host, &installation, &original).await;
    installation.remove_after_services_stopped();
}

#[tokio::test(flavor = "multi_thread")]
async fn owned_tui_bootstrap_cold_profile_reuses_exact_confirmed_services() {
    let installation = Installation::new(None);
    let first = yolo_tui_local_startup::prepare(&installation.profile, &installation.tui)
        .await
        .unwrap();
    let services = Services::capture(&installation.profile.host_home, &first).await;
    let second = yolo_tui_local_startup::prepare(&installation.profile, &installation.tui)
        .await
        .unwrap();
    assert_eq!(first, second);
    let (host, hub) = Services::inspect(&installation.profile.host_home, &second).await;
    assert_eq!(host, services.host);
    assert_eq!(hub, services.hub);
    services.stop().await;
    installation.remove_after_services_stopped();
}
