use super::{backend::Backend, constants::*, fixture::*, model::*};
use std::sync::Arc;
use yolo_hub_client_access_runtime::ClientAccessRuntime;
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{CLIENT_ACCESS_METHODS, Command, Method, Reply, Request};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn hello_advertises_client_access_once_and_preserves_inner_methods() {
    let fixture = Fixture::new();
    let Reply::Hello { hello } = fixture.call(Command::Hello {}).await.unwrap() else {
        panic!()
    };
    assert_eq!(hello.hub_id, fixture.state.hub_id());
    for method in [Method::Hello, Method::List]
        .into_iter()
        .chain(CLIENT_ACCESS_METHODS.iter().copied())
    {
        assert_eq!(
            hello
                .methods
                .iter()
                .filter(|value| **value == method)
                .count(),
            ONE
        );
    }
    assert_eq!(fixture.backend.calls().len(), ONE);
    let state = fixture.state.clone();
    drop(fixture.runtime);
    let mut backend = Backend::new(state.hub_id());
    backend.methods.extend_from_slice(CLIENT_ACCESS_METHODS);
    let runtime = ClientAccessRuntime::open(state, Arc::new(backend), None).unwrap();
    let Reply::Hello { hello } = call(runtime.as_ref(), Command::Hello {}).await.unwrap() else {
        panic!()
    };
    for method in CLIENT_ACCESS_METHODS {
        assert_eq!(
            hello
                .methods
                .iter()
                .filter(|value| *value == method)
                .count(),
            ONE
        );
    }
}

#[tokio::test]
async fn unrelated_requests_preserve_the_complete_request_and_inner_errors() {
    let fixture = Fixture::new();
    let request = Request::new(
        HubId::new(),
        Command::List {
            include_archived: true,
            after: None,
        },
    )
    .unwrap();
    assert!(matches!(
        fixture.runtime.dispatch(request.clone()).await,
        Ok(Operation::Reply(Reply::Terminals { .. }))
    ));
    assert_eq!(fixture.backend.calls(), vec![request]);
    assert!(matches!(
        fixture.call(Command::GetConfiguration {}).await,
        Err(PublicError::Unsupported)
    ));
}

#[tokio::test]
async fn access_management_rejects_foreign_destinations_without_touching_inner_dispatcher() {
    let fixture = Fixture::new();
    let current = fixture.current().await;
    let before = fixture.backend.calls();
    let foreign = HubId::new();
    let mut proposal = change(&current);
    proposal.configuration.entry_hub = foreign;
    for command in [
        Command::GetClientAccess {},
        Command::ConfigureClientAccess { change: proposal },
    ] {
        let request = Request::new(foreign, command).unwrap();
        assert!(matches!(
            fixture.runtime.dispatch(request).await,
            Err(PublicError::Unauthorized)
        ));
    }
    assert_eq!(fixture.backend.calls(), before);
    assert_eq!(fixture.current().await, current);
}

#[test]
fn opening_rejects_an_inner_dispatcher_with_another_hub_identity() {
    let fixture = Fixture::new();
    let state = fixture.state.clone();
    drop(fixture.runtime);
    assert!(
        ClientAccessRuntime::open(state.clone(), Arc::new(Backend::new(HubId::new())), None)
            .is_err()
    );
    assert!(ClientAccessRuntime::open(state, fixture.backend.clone(), None).is_ok());
}

#[test]
fn last_runtime_drop_revokes_admission_even_when_a_registry_handle_is_retained() {
    let fixture = Fixture::new();
    let registry = fixture.runtime.registry();
    let admission = registry
        .admit_verified_key(&client_key(), u64::MIN)
        .unwrap();
    assert!(admission.is_current(u64::MIN));
    let retained = fixture.runtime.clone();
    drop(fixture.runtime);
    assert!(admission.is_current(u64::MIN));
    drop(retained);
    assert!(!admission.is_current(u64::MIN));
    assert!(registry.current().is_err());
}
