use super::support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY, acquire, deadline};
use super::tui_queries::{constants as query, driver::Driver, source::Source};
use crate::Harness;
use yolo_hub_rpc_protocol::{Command, Reply};
#[path = "tui_clipboard_constants.rs"]
mod constants;
#[path = "tui_clipboard_gesture.rs"]
mod gesture;
#[path = "tui_clipboard_lease.rs"]
mod lease;
#[path = "tui_clipboard_probe.rs"]
mod probe;
#[path = "tui_clipboard_runtime.rs"]
mod runtime;
use constants::*;
use probe::Phase;

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_clipboard_is_explicit_passive_frozen_and_fenced_from_provider_and_old_runtime() {
    let host = Harness::start().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let hub = RunningHub::start(
        &host,
        &root.path().canonicalize().unwrap().join(STATE_DIRECTORY),
    )
    .await;
    let mut source = Source::start(&host, hub.hub, query::FIRST_NAME).await;
    let holder = lease::Holder::start(&hub, source.target).await;
    let driver_host = Harness::start().await;
    let mut driver = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
    driver.open(source.name, query::INPUT_HELD).await;
    let probe = probe::Probe::start(driver.clipboard_output().await);
    assert!(source.emit(SOURCE_OUTPUT).await.is_empty());
    driver.wait(|text| text.contains(SOURCE_MARKER)).await;
    probe
        .assert_commands(Phase::SourceOutput, &[], &driver)
        .await;
    let anchor = gesture::begin(&mut driver).await;
    assert!(source.emit(LIVE_OUTPUT).await.is_empty());
    gesture::finish(&mut driver, anchor).await;
    assert!(driver.screen().contents().contains(SOURCE_MARKER));
    assert!(!driver.screen().contents().contains(LIVE_MARKER));
    driver.send(COPY).await;
    driver
        .wait(|text| text.contains(COPY_LABEL) && text.contains(LIVE_MARKER))
        .await;
    probe
        .assert_commands(Phase::ExplicitCopy, &[EXPECTED_COMMAND], &driver)
        .await;
    assert!(source.emit(PROVIDER_CLIPBOARD).await.is_empty());
    assert!(source.emit(SOURCE_OUTPUT).await.is_empty());
    driver.wait(|text| text.contains(SOURCE_MARKER)).await;
    probe
        .assert_commands(Phase::ProviderOutput, &[EXPECTED_COMMAND], &driver)
        .await;

    let anchor = gesture::begin(&mut driver).await;
    driver.send(FOCUS_LOST).await;
    gesture::mouse(&driver, MOUSE_RELEASE, anchor.0, anchor.1, MOUSE_UP).await;
    driver.send(COPY).await;
    driver.wait(|text| text.contains(NO_SELECTION_LABEL)).await;
    probe
        .assert_commands(Phase::FocusLost, &[EXPECTED_COMMAND], &driver)
        .await;
    assert!(source.emit(query::EMPTY_OUTPUT).await.is_empty());
    let observed_reply = hub
        .client()
        .await
        .call(
            hub.hub,
            Command::ObserveLease {
                target: source.target,
            },
            deadline(),
        )
        .await
        .unwrap();
    let Reply::LeaseObservation {
        lease: Some(observed),
    } = observed_reply
    else {
        panic!("external clipboard holder observation returned {observed_reply:?}")
    };
    assert_eq!(
        observed.fields().generation,
        holder.original.fields().generation
    );
    assert_eq!(
        observed.fields().holder_id,
        holder.original.fields().principal.client_id.to_string()
    );

    let anchor = gesture::begin(&mut driver).await;
    gesture::finish(&mut driver, anchor).await;
    let _ = holder.stop().await;
    runtime::replace(&host, &mut driver, source.target).await;
    probe
        .assert_commands(Phase::RuntimeReplaced, &[EXPECTED_COMMAND], &driver)
        .await;
    drop(probe);
    driver.finish().await;
    hub.stop().await;
}
