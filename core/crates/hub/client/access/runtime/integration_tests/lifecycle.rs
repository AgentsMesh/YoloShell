use super::{constants::*, fixture::*, model::*};
use std::{future::Future, task::Poll};
use tokio::sync::watch;

#[tokio::test]
async fn shutdown_flag_or_sender_loss_closes_retained_registry_authority() {
    for initial in [false, true] {
        let fixture = Fixture::new();
        let registry = fixture.runtime.registry();
        let admission = registry
            .admit_verified_key(&client_key(), u64::MIN)
            .unwrap();
        let (shutdown, receiver) = watch::channel(initial);
        let running = tokio::spawn(fixture.runtime.clone().run(receiver));
        if !initial {
            drop(shutdown);
        }
        tokio::time::timeout(TEST_TIMEOUT, running)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(!admission.is_current(u64::MIN));
        assert!(registry.current().is_err());
    }
}

#[tokio::test]
async fn dropping_a_pending_monitor_revokes_without_waiting_for_runtime_drop() {
    let fixture = Fixture::new();
    let registry = fixture.runtime.registry();
    let admission = registry
        .admit_verified_key(&client_key(), u64::MIN)
        .unwrap();
    let (_shutdown, receiver) = watch::channel(false);
    let mut running = Box::pin(fixture.runtime.clone().run(receiver));
    std::future::poll_fn(|context| {
        assert!(running.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(admission.is_current(u64::MIN));
    drop(running);
    assert!(!admission.is_current(u64::MIN));
    tokio::time::timeout(TEST_TIMEOUT, admission.revoked())
        .await
        .unwrap();
    assert!(registry.current().is_err());
}

#[derive(Clone, Copy)]
enum LostAuthority {
    Store,
    Registry,
    Hub,
}

#[tokio::test]
async fn monitor_detects_store_registry_and_hub_loss_without_a_management_request() {
    for loss in [
        LostAuthority::Store,
        LostAuthority::Registry,
        LostAuthority::Hub,
    ] {
        let fixture = Fixture::new();
        let registry = fixture.runtime.registry();
        let admission = registry
            .admit_verified_key(&client_key(), u64::MIN)
            .unwrap();
        let record = fixture.record();
        let hub_directory = fixture.state.directory().unwrap().path().to_owned();
        let (_shutdown, receiver) = watch::channel(false);
        let running = tokio::spawn(fixture.runtime.clone().run(receiver));
        match loss {
            LostAuthority::Store => std::fs::write(&record, INVALID_BYTES).unwrap(),
            LostAuthority::Registry => registry.close(),
            LostAuthority::Hub => {
                std::fs::rename(hub_directory, fixture.sibling(RETIRED_HUB)).unwrap()
            }
        }
        assert!(
            tokio::time::timeout(TEST_TIMEOUT, running)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        assert!(!admission.is_current(u64::MIN));
        assert!(registry.current().is_err());
    }
}

#[tokio::test]
async fn explicit_shutdown_of_an_already_waiting_monitor_revokes_existing_admissions() {
    let fixture = Fixture::new();
    let registry = fixture.runtime.registry();
    let admission = registry
        .admit_verified_key(&client_key(), u64::MIN)
        .unwrap();
    let (shutdown, receiver) = watch::channel(false);
    let mut running = Box::pin(fixture.runtime.clone().run(receiver));
    std::future::poll_fn(|context| {
        assert!(running.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    shutdown.send_replace(true);
    tokio::time::timeout(TEST_TIMEOUT, running)
        .await
        .unwrap()
        .unwrap();
    assert!(!admission.is_current(u64::MIN));
    assert!(registry.current().is_err());
}
