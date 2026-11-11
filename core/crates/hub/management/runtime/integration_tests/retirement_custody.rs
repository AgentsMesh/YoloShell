#[path = "../test_support/retirement_constants.rs"]
mod constants;
#[path = "../test_support/retirement_gate.rs"]
mod support;

use super::fixture::{Fixture, change};
use constants::*;
use std::sync::Arc;
use support::RetirementGate;
use tokio::time::{Instant, timeout, timeout_at};

#[tokio::test]
async fn configuration_waits_for_the_previous_generation_completion_before_replacement() {
    let mut fixture = Fixture::new().await;
    let before = fixture.service.snapshot().unwrap();
    let update = change(&before);
    let request_id = update.request_id;
    let mut gate = RetirementGate::install(&mut fixture.service);
    let mut applying = Box::pin(fixture.service.change(update));
    tokio::select! {
        result = &mut applying => panic!("{COMPLETED_BEFORE_RELEASE}: {result:?}"),
        result = timeout(TEST_TIMEOUT, &mut gate.stopped) => result.unwrap().unwrap(),
    }
    assert!(gate.previous.upgrade().is_some());
    assert!(fixture.hub.generation.read().await.is_none());

    gate.release();
    let applied = timeout(TEST_TIMEOUT, applying).await.unwrap().unwrap();
    assert!(applied.applied);
    assert_eq!(applied.state.last_change, request_id);
    assert!(applied.state.revision > before.state.revision);
    assert!(gate.previous.upgrade().is_none());
    assert!(fixture.hub.generation.read().await.is_some());
    assert!(fixture.service.retiring.is_none());
    timeout(TEST_TIMEOUT, fixture.service.retire())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn expired_retirement_wait_retains_network_completion_for_configuration_retry() {
    let mut fixture = Fixture::new().await;
    let before = fixture.service.snapshot().unwrap();
    let update = change(&before);
    let mut gate = RetirementGate::install(&mut fixture.service);
    let mut applying = Box::pin(fixture.service.change(update.clone()));
    tokio::select! {
        result = &mut applying => panic!("{COMPLETED_BEFORE_RELEASE}: {result:?}"),
        result = timeout(TEST_TIMEOUT, &mut gate.stopped) => result.unwrap().unwrap(),
    }
    assert!(timeout_at(Instant::now(), applying).await.is_err());
    assert!(fixture.service.running.is_none());
    assert!(fixture.service.retiring.is_some());
    assert!(fixture.hub.generation.read().await.is_none());
    assert!(gate.previous.upgrade().is_some());
    let pending = fixture.service.snapshot().unwrap();
    assert_eq!(pending.state, before.state);
    assert!(!pending.applied);

    gate.release();
    let applied = timeout(TEST_TIMEOUT, fixture.service.change(update.clone()))
        .await
        .unwrap()
        .unwrap();
    assert!(applied.applied);
    assert_eq!(applied.state.last_change, update.request_id);
    assert!(gate.previous.upgrade().is_none());
    assert!(fixture.service.retiring.is_none());
    assert!(fixture.hub.generation.read().await.is_some());
    let replay = fixture.service.change(update).await.unwrap();
    assert_eq!(replay, applied);
    timeout(TEST_TIMEOUT, fixture.service.retire())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn cancelling_before_generation_lock_acquisition_retains_the_old_join_owner() {
    let mut fixture = Fixture::new().await;
    let before = fixture.service.snapshot().unwrap();
    let running = fixture.service.running.as_ref().unwrap();
    let previous = Arc::downgrade(&running.node);
    let mut stopping = running.stop.subscribe();
    let reader = fixture.hub.generation.read().await;
    let mut retiring = Box::pin(fixture.service.retire());
    tokio::select! {
        result = &mut retiring => panic!("{RETIRED_BEFORE_LOCK_RELEASE}: {result:?}"),
        result = timeout(TEST_TIMEOUT, stopping.changed()) => result.unwrap().unwrap(),
    }
    drop(retiring);
    assert!(fixture.service.running.is_none());
    assert!(fixture.service.retiring.is_some());
    assert!(reader.is_some());
    assert!(previous.upgrade().is_some());
    drop(reader);

    let applied = timeout(TEST_TIMEOUT, fixture.service.change(change(&before)))
        .await
        .unwrap()
        .unwrap();
    assert!(applied.applied);
    assert!(previous.upgrade().is_none());
    assert!(fixture.service.retiring.is_none());
    assert!(fixture.hub.generation.read().await.is_some());
    timeout(TEST_TIMEOUT, fixture.service.retire())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn failed_network_completion_survives_a_retained_generation_postcondition_retry() {
    let mut fixture = Fixture::new().await;
    let before = fixture.service.snapshot().unwrap();
    let retained = fixture.service.running.as_ref().unwrap().node.clone();
    let mut gate = RetirementGate::install_failed(&mut fixture.service);
    gate.release();
    let rejected = timeout(TEST_TIMEOUT, fixture.service.retire())
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(rejected.to_string(), crate::constants::GENERATION_RETAINED);
    assert!(fixture.service.retiring.is_some());
    assert!(fixture.hub.generation.read().await.is_none());
    assert_eq!(fixture.service.snapshot().unwrap().state, before.state);

    drop(retained);
    let completed = timeout(TEST_TIMEOUT, fixture.service.retire())
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(completed.to_string(), NETWORK_COMPLETION_FAILURE);
    assert!(fixture.service.retiring.is_none());
    assert!(gate.previous.upgrade().is_none());
    let recovered = timeout(TEST_TIMEOUT, fixture.service.change(change(&before)))
        .await
        .unwrap()
        .unwrap();
    assert!(recovered.applied);
    timeout(TEST_TIMEOUT, fixture.service.retire())
        .await
        .unwrap()
        .unwrap();
}
