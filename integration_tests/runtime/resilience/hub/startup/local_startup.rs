use super::local_startup_support::*;
#[path = "local_startup_connector.rs"]
mod connector;
#[path = "local_startup_races.rs"]
mod races;
#[path = "local_startup_session.rs"]
mod session;

#[tokio::test(flavor = "multi_thread")]
async fn real_local_bootstrap_starts_services_and_reuses_the_same_identity_after_launcher_exit() {
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let root = root.path().canonicalize().unwrap();
    let home = root.join(HOST_DIRECTORY);
    let state = root.join(STATE_DIRECTORY);
    let first = bootstrap(&home, &state).await;
    let services = Services::capture(&home, &first).await;
    assert!(services.host.matches() && services.hub.matches());
    connector::assert_empty(&first).await;
    let second = bootstrap(&home, &state).await;
    assert_eq!(second, first);
    let (same_host, same_hub) = Services::inspect(&home, &second).await;
    assert_eq!(same_host, services.host);
    assert_eq!(same_hub, services.hub);
    connector::assert_empty(&second).await;
    services.stop().await;
    let third = bootstrap(&home, &state).await;
    assert_eq!(third.entry_hub, first.entry_hub);
    let restarted = Services::capture(&home, &third).await;
    connector::assert_empty(&third).await;
    restarted.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_cold_bootstraps_reuse_one_host_and_one_hub_identity() {
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let path = root.path().canonicalize().unwrap();
    let home = path.join(HOST_DIRECTORY);
    let state = path.join(STATE_DIRECTORY);
    let (first, second) = tokio::join!(bootstrap(&home, &state), bootstrap(&home, &state));
    let services = Services::capture(&home, &first).await;
    assert_eq!(first, second);
    let (host, hub) = Services::inspect(&home, &second).await;
    assert_eq!(host, services.host);
    assert_eq!(hub, services.hub);
    connector::assert_empty(&second).await;
    services.stop().await;
}
