use uuid::Uuid;
use yolo_hub_client_access_protocol::*;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Method, Reply, Request, Response};

fn configuration() -> AccessConfiguration {
    AccessConfiguration {
        schema_version: ACCESS_SCHEMA_VERSION,
        entry_hub: HubId::new(),
        revision: FIRST_ACCESS_REVISION + ACCESS_REVISION_STEP,
        clients: vec![],
    }
}

#[test]
fn client_policy_management_is_local_only_and_correlates_complete_replacement() {
    let configuration = configuration();
    let change = ClientAccessChange {
        request_id: Uuid::new_v4(),
        expected_revision: FIRST_ACCESS_REVISION,
        configuration: configuration.clone(),
    };
    let command = Command::ConfigureClientAccess {
        change: change.clone(),
    };
    assert!(command.local_only() && command.mutates());
    assert_eq!(command.request_id(), Some(change.request_id));
    assert_eq!(Method::ConfigureClientAccess.required_rights(), None);
    assert_eq!(Method::GetClientAccess.required_rights(), None);
    assert!(Command::GetClientAccess {}.local_only());
    assert!(!Command::GetClientAccess {}.mutates());
    let request = Request::new(configuration.entry_hub, command).unwrap();
    let mut response = Response::new(
        &request,
        configuration.entry_hub,
        Reply::ClientAccess {
            configuration: configuration.clone(),
        },
    );
    response.validate_for(&request).unwrap();
    let Reply::ClientAccess {
        configuration: returned,
    } = &mut response.reply
    else {
        panic!()
    };
    returned.revision += ACCESS_REVISION_STEP;
    assert!(response.validate_for(&request).is_err());
    let command = Command::ConfigureClientAccess { change };
    assert!(Request::new(HubId::new(), command).is_err());
}

#[test]
fn invalid_revision_and_nil_change_ids_cannot_enter_the_wire() {
    let mut change = ClientAccessChange {
        request_id: Uuid::new_v4(),
        expected_revision: FIRST_ACCESS_REVISION,
        configuration: configuration(),
    };
    change.request_id = Uuid::nil();
    assert!(change.validate().is_err());
    change.request_id = Uuid::new_v4();
    change.expected_revision = u64::MAX;
    assert!(change.validate().is_err());
    change.expected_revision = u64::default();
    assert!(change.validate().is_err());
}
