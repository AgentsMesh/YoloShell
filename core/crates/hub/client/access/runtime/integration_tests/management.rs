use super::{constants::*, fixture::*, model::*};
use yolo_hub_client_access_runtime::ClientAccessRuntime;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_rpc_protocol::{Command, Reply, Request};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn committed_policy_replaces_registry_revokes_old_admission_and_survives_restart() {
    let fixture = Fixture::new();
    let original = fixture.current().await;
    let registry = fixture.runtime.registry();
    let admission = registry
        .admit_verified_key(&client_key(), u64::MIN)
        .unwrap();
    let mut proposed = change(&original);
    proposed.configuration.clients.clear();
    let expected = proposed.configuration.clone();
    assert_eq!(
        fixture
            .call(Command::ConfigureClientAccess { change: proposed })
            .await
            .unwrap(),
        Reply::ClientAccess {
            configuration: expected.clone()
        }
    );
    assert_eq!(fixture.current().await, expected);
    assert_eq!(registry.current().unwrap(), expected);
    assert!(!admission.is_current(u64::MIN));
    tokio::time::timeout(TEST_TIMEOUT, admission.revoked())
        .await
        .unwrap();
    assert!(
        registry
            .admit_verified_key(&client_key(), u64::MIN)
            .is_err()
    );
    let state = fixture.state.clone();
    drop(fixture.runtime);
    let runtime = ClientAccessRuntime::open(state, fixture.backend.clone(), None).unwrap();
    assert_eq!(
        call(runtime.as_ref(), Command::GetClientAccess {})
            .await
            .unwrap(),
        Reply::ClientAccess {
            configuration: expected
        }
    );
}

#[tokio::test]
async fn stale_cas_and_foreign_configuration_do_not_poison_later_valid_updates() {
    let fixture = Fixture::new();
    let original = fixture.current().await;
    let registry = fixture.runtime.registry();
    let admission = registry
        .admit_verified_key(&client_key(), u64::MIN)
        .unwrap();
    let bytes = std::fs::read(fixture.record()).unwrap();
    let mut stale = change(&original);
    stale.expected_revision += REVISION_STEP;
    stale.configuration.revision += REVISION_STEP;
    assert!(matches!(
        fixture
            .call(Command::ConfigureClientAccess { change: stale })
            .await,
        Err(PublicError::Conflict)
    ));
    let mut foreign = change(&original);
    foreign.configuration.entry_hub = yolo_hub_identity::HubId::new();
    assert!(matches!(
        Request::new(
            fixture.runtime.hub_id(),
            Command::ConfigureClientAccess { change: foreign }
        ),
        Err(PublicError::OriginMismatch)
    ));
    assert_eq!(std::fs::read(fixture.record()).unwrap(), bytes);
    assert_eq!(fixture.current().await, original);
    assert!(admission.is_current(u64::MIN));
    let proposed = change(&original);
    let expected = proposed.configuration.clone();
    assert_eq!(
        fixture
            .call(Command::ConfigureClientAccess { change: proposed })
            .await
            .unwrap(),
        Reply::ClientAccess {
            configuration: expected.clone()
        }
    );
    assert_eq!(registry.current().unwrap(), expected);
    assert!(!admission.is_current(u64::MIN));
}

#[tokio::test]
async fn stale_exact_replay_is_a_conflict_and_does_not_rewrite_committed_bytes() {
    let fixture = Fixture::new();
    let proposed = change(&fixture.current().await);
    fixture
        .call(Command::ConfigureClientAccess {
            change: proposed.clone(),
        })
        .await
        .unwrap();
    let expected = fixture.current().await;
    let bytes = std::fs::read(fixture.record()).unwrap();
    assert!(matches!(
        fixture
            .call(Command::ConfigureClientAccess { change: proposed })
            .await,
        Err(PublicError::Conflict)
    ));
    assert_eq!(std::fs::read(fixture.record()).unwrap(), bytes);
    assert_eq!(fixture.current().await, expected);
}
