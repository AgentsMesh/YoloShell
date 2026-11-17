#[path = "../test_support/dialer_constants.rs"]
mod constants;
#[path = "../test_support/pending_dialer.rs"]
mod support;
use crate::*;
use constants::*;
use std::{num::NonZeroU16, sync::atomic::Ordering};
use support::{Event, PendingDialer};
use tokio::time::{Instant, timeout};
use yolo_hub_federation_settings::{Peer, PeerAddress};

#[tokio::test]
async fn configuration_generation_changes_reuse_the_injected_factory_and_cancel_old_dials() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let home = root.join(HOST_DIRECTORY);
    let state = Arc::new(HubState::open(&root.join(HUB_DIRECTORY), &home).unwrap());
    let remote = HubState::open(
        &root.join(REMOTE_DIRECTORY),
        &root.join(REMOTE_HOME_DIRECTORY),
    )
    .unwrap();
    let identity = remote.peer_identity();
    let first_address = PeerAddress::tcp(DIALER_HOST, NonZeroU16::new(DIALER_PORT).unwrap());
    let mut config = Configuration::default();
    config.peers.push(Peer {
        identity,
        dial: Some(first_address.clone()),
        consume: false,
        exports: vec![],
    });
    let backend = LocalBackend::new(state.hub_id(), &home, &root.join(ABSENT_SOCKET)).unwrap();
    let (factory, mut events) = PendingDialer::new();
    let factory = Arc::new(factory);
    let (hub, mut service) =
        ManagedHub::open_with_dialer(state, backend, Some(config), None, factory.clone())
            .await
            .unwrap();
    let first_generation = Arc::downgrade(&service.running.as_ref().unwrap().node);
    let Event::Started {
        id,
        peer,
        address,
        deadline,
    } = timeout(DIALER_TEST_TIMEOUT, events.recv())
        .await
        .unwrap()
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(id, FIRST_DIAL);
    assert_eq!(peer, identity);
    assert_eq!(address, first_address);
    assert!(deadline <= Instant::now() + yolo_hub_federation::CONNECT_TIMEOUT);
    let previous = service.snapshot().unwrap();
    let mut change = super::fixture::change(&previous);
    let next_address = PeerAddress::tcp(REPLACEMENT_HOST, NonZeroU16::new(DIALER_PORT).unwrap());
    change.configuration.peers[0].dial = Some(next_address.clone());
    let applied = timeout(DIALER_TEST_TIMEOUT, service.change(change))
        .await
        .unwrap()
        .unwrap();
    assert!(applied.applied);
    assert!(first_generation.upgrade().is_none());
    assert!(matches!(
        timeout(DIALER_TEST_TIMEOUT, events.recv())
            .await
            .unwrap()
            .unwrap(),
        Event::Closed(FIRST_DIAL)
    ));
    let Event::Started {
        id, peer, address, ..
    } = timeout(DIALER_TEST_TIMEOUT, events.recv())
        .await
        .unwrap()
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(id, SECOND_DIAL);
    assert_eq!(peer, identity);
    assert_eq!(address, next_address);
    let mut remove = super::fixture::change(&applied);
    remove.configuration.peers.clear();
    assert!(
        timeout(DIALER_TEST_TIMEOUT, service.change(remove))
            .await
            .unwrap()
            .unwrap()
            .applied
    );
    assert!(matches!(
        timeout(DIALER_TEST_TIMEOUT, events.recv())
            .await
            .unwrap()
            .unwrap(),
        Event::Closed(SECOND_DIAL)
    ));
    service.retire().await.unwrap();
    assert!(hub.generation.read().await.is_none());
    assert_eq!(factory.calls.load(Ordering::SeqCst), SECOND_DIAL);
    assert!(events.try_recv().is_err());
}
