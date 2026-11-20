use uuid::Uuid;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::*;
use yolo_hub_rpc_protocol::*;

fn fixture() -> (Request, ConfigurationSnapshot) {
    const KEY_BYTE: u8 = 1;
    let identity = PeerIdentity::new(HubId::new(), [KEY_BYTE; ED25519_PUBLIC_KEY_BYTES]).unwrap();
    let change = ConfigurationChange {
        request_id: Uuid::new_v4(),
        expected_revision: FIRST_CONFIGURATION_REVISION,
        configuration: Configuration::default(),
    };
    let snapshot = ConfigurationSnapshot {
        state: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity,
            revision: change.expected_revision + CONFIGURATION_REVISION_STEP,
            last_change: change.request_id,
            configuration: change.configuration.clone(),
        },
        applied: true,
        peer_socket: Some("/run/user/1000/yolo/hub/peer.sock".into()),
    };
    (
        Request::new(identity.hub(), Command::Configure { change }).unwrap(),
        snapshot,
    )
}

#[test]
fn configure_response_requires_the_exact_applied_revision_request_and_payload() {
    let (request, snapshot) = fixture();
    let response = |snapshot| {
        Response::new(
            &request,
            request.fields().destination,
            Reply::Configuration { snapshot },
        )
    };
    response(snapshot.clone()).validate_for(&request).unwrap();
    let mutations: &[fn(&mut ConfigurationSnapshot)] = &[
        |snapshot| snapshot.applied = false,
        |snapshot| snapshot.state.revision += CONFIGURATION_REVISION_STEP,
        |snapshot| snapshot.state.last_change = Uuid::new_v4(),
        |snapshot| snapshot.peer_socket = None,
        |snapshot| snapshot.state.schema_version = u16::MAX,
    ];
    for mutate in mutations {
        let mut changed = snapshot.clone();
        mutate(&mut changed);
        assert!(response(changed).validate_for(&request).is_err());
    }
}

#[test]
fn get_configuration_can_report_unapplied_state_but_cannot_substitute_another_hub() {
    let (original, mut snapshot) = fixture();
    let request =
        Request::new(original.fields().destination, Command::GetConfiguration {}).unwrap();
    snapshot.applied = false;
    snapshot.peer_socket = None;
    Response::new(
        &request,
        request.fields().destination,
        Reply::Configuration {
            snapshot: snapshot.clone(),
        },
    )
    .validate_for(&request)
    .unwrap();
    snapshot.state.identity =
        PeerIdentity::new(HubId::new(), *snapshot.state.identity.key()).unwrap();
    assert!(
        Response::new(
            &request,
            request.fields().destination,
            Reply::Configuration { snapshot }
        )
        .validate_for(&request)
        .is_err()
    );
}

#[test]
fn configuration_commands_are_local_only_with_mutation_identity_and_revision_fences() {
    let (request, _) = fixture();
    let command = &request.fields().command;
    assert!(command.local_only() && command.mutates());
    assert_eq!(command.request_id(), Some(request.fields().request_id));
    let query = Command::GetConfiguration {};
    assert!(query.local_only() && !query.mutates());
    assert_eq!(query.request_id(), None);
    let Command::Configure { mut change } = command.clone() else {
        panic!()
    };
    change.expected_revision = u64::MIN;
    assert!(
        Request::new(
            request.fields().destination,
            Command::Configure {
                change: change.clone()
            }
        )
        .is_err()
    );
    change.expected_revision = FIRST_CONFIGURATION_REVISION;
    change.request_id = Uuid::nil();
    assert!(Request::new(request.fields().destination, Command::Configure { change }).is_err());
}

#[test]
fn consumers_reply_is_local_only_and_roundtrips() {
    let destination = HubId::new();
    let request = Request::new(destination, Command::ListConsumers {}).unwrap();
    assert!(!request.fields().command.mutates());
    let reply = Reply::Consumers {
        consumers: vec![ConsumerSummary {
            hub: HubId::new(),
            connected_since_ms: 1,
            links: 1,
        }],
    };
    let response = Response::new(&request, destination, reply);
    response.validate_for(&request).unwrap();
    let bytes = serde_json::to_vec(&response).unwrap();
    assert_eq!(serde_json::from_slice::<Response>(&bytes).unwrap(), response);
}
