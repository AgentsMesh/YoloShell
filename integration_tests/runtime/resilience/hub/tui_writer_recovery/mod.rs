mod constants;
mod fixture;
use crate::hub::tui_queries::constants::EMPTY_OUTPUT;
use constants::*;
use fixture::Fixture;
use yolo_runtime::{lease::constants::MAX_TTL_MS, model::WriterCapability};

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_recovers_writer_after_hub_restart_without_navigation_or_input_replay() {
    let mut fixture = Fixture::start().await;
    fixture.interrupt_hub().await;
    fixture.restart_hub().await;
    fixture.recovered().await;
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_recovery_waits_for_competing_writer_release_without_takeover_or_replay() {
    let mut fixture = Fixture::start().await;
    let client = fixture.host.client();
    let runtime = fixture.runtime();
    fixture.interrupt_hub().await;
    let observed = client.get_writer_lease(runtime).await.unwrap();
    let grant = client
        .take_over_writer_for_instance(
            runtime,
            COMPETING_HOLDER,
            None,
            observed.map(|lease| lease.generation),
            WriterCapability::generate(),
            Some(MAX_TTL_MS),
        )
        .await
        .unwrap();
    fixture.restart_hub().await;
    fixture.wait_status(INPUT_HELD).await;
    fixture.driver.send(CONTESTED_INPUT).await;
    assert_eq!(
        fixture.source.emit(RECONNECTED_OUTPUT).await,
        BASELINE_INPUT
    );
    fixture
        .driver
        .wait(|text| text.contains(RECONNECTED_MARKER))
        .await;
    let held = client.get_writer_lease(runtime).await.unwrap().unwrap();
    assert_eq!(held.holder_id, COMPETING_HOLDER);
    assert_eq!(held.generation, grant.generation);
    assert_eq!(fixture.source.emit(EMPTY_OUTPUT).await, BASELINE_INPUT);
    client.release_writer(&grant, runtime).await.unwrap();
    fixture.recovered().await;
    fixture.finish().await;
}
