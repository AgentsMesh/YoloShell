use super::super::local_startup_support::*;
#[path = "local_startup_cancellation.rs"]
mod cancellation;
#[path = "local_startup_parent_loss.rs"]
mod parent_loss;
#[path = "local_startup_race_support.rs"]
mod support;
use support::*;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_local_startup::ensure_running;
use yolo_hub_server::HubServer;
use yolo_hub_state::HubState;

#[tokio::test(flavor = "multi_thread")]
async fn slow_hub_initialization_keeps_the_original_empty_host_alive() {
    let directory = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let root = directory.path().canonicalize().unwrap();
    let home = root.join(HOST_DIRECTORY);
    let state = root.join(STATE_DIRECTORY);
    let executable = wrapper(&root, &format!("{DELAY_PREFIX}{DELAY_SECONDS}{DELAY_EXEC}"));
    let startup = tokio::spawn(ensure_running(
        launch(&home, &state, executable),
        deadline(),
    ));
    let socket = yolo_runtime::paths::Paths::new(&home)
        .host_socket()
        .unwrap();
    let client = yolo_client::Client::managed_local(&home, &socket).unwrap();
    let first = tokio::time::timeout_at(deadline(), async {
        loop {
            if let Ok(status) = client.status().await {
                break crate::ProcessIdentity::capture(status.host_pid).unwrap();
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    let document = startup.await.unwrap().unwrap();
    let services = Services::capture(&home, &document).await;
    assert_eq!(services.host, first);
    assert!(first.matches());
    services.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_waits_for_the_existing_hub_after_its_contender_loses_the_singleton() {
    existing_hub_contender_waits(std::time::Duration::ZERO).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn delayed_hub_fixture_setup_preserves_the_original_host_for_singleton_race() {
    existing_hub_contender_waits(SLOW_FIXTURE_SETUP).await;
}

async fn existing_hub_contender_waits(setup_delay: std::time::Duration) {
    let harness = crate::Harness::start().await;
    let setup_hold = retain_fixture_host(&harness).await;
    let setup_started = std::time::Instant::now();
    tokio::time::sleep(setup_delay).await;
    let directory = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let root = directory.path().canonicalize().unwrap();
    let state_path = root.join(STATE_DIRECTORY);
    let state = HubState::open(&state_path, harness.home.path()).unwrap();
    let expected = state.hub_id();
    let executable = wrapper(&root, CONTENDER_SCRIPT);
    let backend = LocalBackend::new(
        expected,
        harness.home.path(),
        &harness.paths().host_socket().unwrap(),
    )
    .unwrap();
    let _hold = backend.retain_host().await.unwrap_or_else(|error| {
        panic!(
            "{RETAIN_HOST_FAILURE}: {error:?}; elapsed={:?}; host={:?}; alive={}; stderr={}",
            setup_started.elapsed(),
            harness.host_identity(),
            harness.host_identity().matches(),
            harness.host_stderr_tail()
        )
    });
    assert_eq!(_hold.initial_host_pid(), harness.host_identity().pid);
    assert!(harness.host_identity().matches());
    drop(setup_hold);
    let marker = root.join(CONTENDER_EXIT);
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let serving = tokio::spawn(async move {
        wait_file(&marker).await;
        HubServer::bind(state, backend)
            .await
            .unwrap()
            .serve_until(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let document = ensure_running(
        launch(harness.home.path(), &state_path, executable),
        deadline(),
    )
    .await
    .unwrap();
    assert_eq!(document.entry_hub, expected);
    stop.send(()).unwrap();
    tokio::time::timeout_at(deadline(), serving)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_waits_for_the_existing_host_after_its_contender_loses_the_singleton() {
    let directory = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let root = directory.path().canonicalize().unwrap();
    let home = root.join(HOST_DIRECTORY);
    let state = root.join(STATE_DIRECTORY);
    let mut config = yolo_runtime::host::HostConfig::new(&home);
    config.runtime_executable = Some(crate::runtime_binary());
    let host = yolo_runtime::host::Host::open(config).unwrap();
    let marker = root.join(CONTENDER_EXIT);
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let serving = tokio::spawn(async move {
        wait_file(&marker).await;
        host.serve_with_stop(async {
            let _ = stopped.await;
        })
        .await
        .unwrap();
    });
    let mut startup = launch(&home, &state, hub_binary());
    startup.host_executable = host_wrapper(&root, CONTENDER_SCRIPT);
    let document = ensure_running(startup, deadline()).await.unwrap();
    let (owner, hub) = Services::inspect(&home, &document).await;
    assert_eq!(owner.pid, std::process::id());
    let hub = ExactCleanup(hub);
    hub.0.signal(libc::SIGTERM).unwrap();
    crate::wait_for_identity_gone(hub.0).await;
    let _ = stop.send(());
    tokio::time::timeout_at(deadline(), serving)
        .await
        .unwrap()
        .unwrap();
}
