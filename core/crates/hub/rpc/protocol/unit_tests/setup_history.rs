use yolo_hub_identity::HubId;
use yolo_hub_remote_setup_protocol::{
    RemoteSetupHistory, SETUP_OPERATION_ID_FIELD, SetupCheckpoint,
};
use yolo_hub_rpc_protocol::{
    Command, INSPECT_REMOTE_SETUP_METHOD, Method, REMOTE_SETUP_METHODS, RPC_METHOD_TAG,
    RPC_VERSION_FIELD, Reply, Request, Response, SUPPORTED_METHODS,
};

const PREVIOUS_RPC_VERSION: u16 = 9;

#[test]
fn history_capability_requires_a_coordinated_rpc_upgrade() {
    let owner = HubId::new();
    let request = Request::new(owner, Command::Hello {}).unwrap();
    let mut previous = serde_json::to_value(request).unwrap();
    previous[RPC_VERSION_FIELD] = serde_json::json!(PREVIOUS_RPC_VERSION);
    assert!(serde_json::from_value::<Request>(previous).is_err());
    assert!(yolo_hub_rpc_protocol::HUB_PROTOCOL_VERSION > PREVIOUS_RPC_VERSION);
}

#[test]
fn history_is_optional_nonmutating_entry_only_and_rejects_nil_operation_ids() {
    let owner = HubId::new();
    let command = Command::InspectRemoteSetup {
        operation_id: uuid::Uuid::new_v4(),
    };
    assert_eq!(command.method(), Method::InspectRemoteSetup);
    assert!(command.local_only());
    assert!(!command.mutates());
    assert!(!command.requires_read_reply_admission());
    assert_eq!(command.request_id(), None);
    assert!(REMOTE_SETUP_METHODS.contains(&command.method()));
    assert!(!SUPPORTED_METHODS.contains(&command.method()));
    let wire = serde_json::to_value(&command).unwrap();
    assert_eq!(serde_json::from_value::<Command>(wire).unwrap(), command);
    assert!(
        Request::new(
            owner,
            Command::InspectRemoteSetup {
                operation_id: uuid::Uuid::nil()
            }
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<Command>(serde_json::json!({
            (RPC_METHOD_TAG): INSPECT_REMOTE_SETUP_METHOD,
            (SETUP_OPERATION_ID_FIELD): uuid::Uuid::nil()
        }))
        .is_err()
    );
}

#[test]
fn history_replies_bind_both_entry_and_operation_including_missing_records() {
    let owner = HubId::new();
    let id = uuid::Uuid::new_v4();
    let request = Request::new(owner, Command::InspectRemoteSetup { operation_id: id }).unwrap();
    for checkpoint in [
        None,
        Some(SetupCheckpoint::Prepared {}),
        Some(SetupCheckpoint::Complete {
            origin: HubId::new(),
        }),
    ] {
        let valid = RemoteSetupHistory::new(id, owner, checkpoint).unwrap();
        let response = Response::new(
            &request,
            owner,
            Reply::RemoteSetupHistory {
                history: valid.clone(),
            },
        );
        response.validate_for(&request).unwrap();
        let encoded = serde_json::to_vec(&response).unwrap();
        serde_json::from_slice::<Response>(&encoded)
            .unwrap()
            .validate_for(&request)
            .unwrap();
        for history in [
            RemoteSetupHistory {
                owner: HubId::new(),
                ..valid.clone()
            },
            RemoteSetupHistory {
                operation_id: uuid::Uuid::new_v4(),
                ..valid.clone()
            },
            RemoteSetupHistory {
                operation_id: uuid::Uuid::nil(),
                ..valid
            },
        ] {
            assert!(
                Response::new(&request, owner, Reply::RemoteSetupHistory { history })
                    .validate_for(&request)
                    .is_err()
            );
        }
    }
    assert!(
        Response::new(&request, owner, Reply::Completed {})
            .validate_for(&request)
            .is_err()
    );
}
