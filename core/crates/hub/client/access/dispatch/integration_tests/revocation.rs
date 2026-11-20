use super::{backend::*, model::*};
use std::sync::{Arc, atomic::Ordering};
use yolo_hub_client_access_dispatch::{ScopedDispatcher, while_authorized};
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn revocation_retires_inflight_client_work_without_replaying_it() {
    let (registry, admission) = authority(&[Right::Observe]);
    let backend = Arc::new(Blocking::default());
    let scoped = ScopedDispatcher::new(backend.clone(), admission).unwrap();
    let task = tokio::spawn(async move {
        scoped
            .dispatch(request(
                ORIGIN,
                Command::Get {
                    address: address(ORIGIN, SESSION),
                },
            ))
            .await
    });
    tokio::time::timeout(TEST_TIMEOUT, backend.started.notified())
        .await
        .unwrap();
    registry.close();
    assert!(matches!(
        tokio::time::timeout(TEST_TIMEOUT, task)
            .await
            .unwrap()
            .unwrap(),
        Err(PublicError::Unauthorized)
    ));
    backend.released.notify_one();
    assert!(!backend.dispatched.load(Ordering::Acquire));
}

#[tokio::test]
async fn changing_policy_invalidates_existing_admissions_even_if_the_key_stays_the_same() {
    let (registry, admission) = authority(&[Right::Observe]);
    let mut next = registry.current().unwrap();
    let before = next.revision;
    next.revision += 1;
    registry.replace(before, next).unwrap();
    assert_eq!(
        while_authorized(&admission, async { Ok(()) }).await,
        Err(PublicError::Unauthorized)
    );
    let fresh = registry
        .admit_verified_key(
            &[KEY_BYTE; yolo_hub_client_access_protocol::ED25519_PUBLIC_KEY_BYTES],
            now(),
        )
        .unwrap();
    assert_eq!(while_authorized(&fresh, async { Ok(()) }).await, Ok(()));
}

#[tokio::test]
async fn dropping_the_registry_retires_waiting_client_work() {
    let (registry, admission) = authority(&[Right::Observe]);
    let task = tokio::spawn(async move {
        while_authorized(
            &admission,
            std::future::pending::<Result<(), PublicError>>(),
        )
        .await
    });
    drop(registry);
    assert_eq!(
        tokio::time::timeout(TEST_TIMEOUT, task)
            .await
            .unwrap()
            .unwrap(),
        Err(PublicError::Unauthorized)
    );
}
