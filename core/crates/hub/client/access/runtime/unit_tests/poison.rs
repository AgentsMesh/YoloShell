use super::{constants::*, fixture::Fixture, model::*};
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::PublicError;

const POISON_MESSAGE: &str = "injected public client access runtime owner failure";

#[tokio::test]
async fn panicking_store_owner_revokes_admission_before_any_later_dispatch() {
    let fixture = Fixture::new();
    let registry = fixture.runtime.registry();
    let admission = registry
        .admit_verified_key(&client_key(), u64::MIN)
        .unwrap();
    let current = fixture.current().await;
    let runtime = fixture.runtime.clone();
    let panicked = std::thread::spawn(move || {
        let _store = runtime.lock_store().unwrap();
        panic!("{POISON_MESSAGE}");
    });
    assert!(panicked.join().is_err());
    assert!(!admission.is_current(u64::MIN));
    tokio::time::timeout(TEST_TIMEOUT, admission.revoked())
        .await
        .unwrap();
    assert_eq!(
        fixture.call(Command::GetClientAccess {}).await,
        Err(PublicError::Unreachable)
    );
    assert_eq!(
        fixture
            .call(Command::ConfigureClientAccess {
                change: change(&current)
            })
            .await,
        Err(PublicError::OutcomeUnknown)
    );
    assert!(registry.current().is_err());
}

#[tokio::test]
async fn invalid_direct_proposals_are_rejected_before_poisoning_storage() {
    let fixture = Fixture::new();
    let current = fixture.current().await;
    let mut invalid = change(&current);
    invalid.request_id = uuid::Uuid::nil();
    assert_eq!(
        fixture.runtime.configure(&invalid),
        Err(PublicError::InvalidRequest)
    );
    assert_eq!(fixture.current().await, current);
    let mut foreign = change(&current);
    foreign.configuration.entry_hub = yolo_hub_identity::HubId::new();
    assert_eq!(
        fixture.runtime.configure(&foreign),
        Err(PublicError::OriginMismatch)
    );
    assert_eq!(fixture.current().await, current);
    assert!(fixture.runtime.configure(&change(&current)).is_ok());
    let request = Request::new(fixture.state.hub_id(), Command::GetClientAccess {}).unwrap();
    assert!(fixture.runtime.dispatch(request).await.is_ok());
}
