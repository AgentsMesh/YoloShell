use yolo_hub_identity::HubId;
use yolo_hub_remote_setup_protocol::{
    RemoteSetupHistory, SETUP_CHECKPOINT_PHASE_TAG, SETUP_HISTORY_CHECKPOINT_FIELD,
    SETUP_HISTORY_OWNER_FIELD, SETUP_OPERATION_ID_FIELD, SetupCheckpoint,
};

#[test]
fn all_recorded_phases_and_absence_round_trip_without_private_request_material() {
    let owner = HubId::new();
    let origin = HubId::new();
    let operation = uuid::Uuid::new_v4();
    for checkpoint in [
        None,
        Some(SetupCheckpoint::Prepared {}),
        Some(SetupCheckpoint::Provisioning {}),
        Some(SetupCheckpoint::Routing { origin }),
        Some(SetupCheckpoint::Complete { origin }),
    ] {
        let history = RemoteSetupHistory::new(operation, owner, checkpoint).unwrap();
        let wire = serde_json::to_value(&history).unwrap();
        let fields: std::collections::BTreeSet<_> = wire
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            fields,
            [
                SETUP_OPERATION_ID_FIELD,
                SETUP_HISTORY_OWNER_FIELD,
                SETUP_HISTORY_CHECKPOINT_FIELD
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            serde_json::from_value::<RemoteSetupHistory>(wire).unwrap(),
            history
        );
        history.validate_for(owner, operation).unwrap();
    }
}

#[test]
fn absent_records_remain_bound_to_the_requested_operation_and_owner() {
    let owner = HubId::new();
    let operation = uuid::Uuid::new_v4();
    let history = RemoteSetupHistory::new(operation, owner, None).unwrap();
    assert!(history.validate_for(HubId::new(), operation).is_err());
    assert!(history.validate_for(owner, uuid::Uuid::new_v4()).is_err());
    assert!(RemoteSetupHistory::new(uuid::Uuid::nil(), owner, None).is_err());
}

#[test]
fn history_cannot_claim_its_own_entry_as_the_remote_installation() {
    let owner = HubId::new();
    for checkpoint in [
        SetupCheckpoint::Routing { origin: owner },
        SetupCheckpoint::Complete { origin: owner },
    ] {
        assert!(RemoteSetupHistory::new(uuid::Uuid::new_v4(), owner, Some(checkpoint)).is_err());
    }
}

#[test]
fn wire_rejects_nil_operations_unknown_fields_and_unknown_phases() {
    let owner = HubId::new();
    let valid = serde_json::to_value(
        RemoteSetupHistory::new(
            uuid::Uuid::new_v4(),
            owner,
            Some(SetupCheckpoint::Prepared {}),
        )
        .unwrap(),
    )
    .unwrap();
    let mut nil = valid.clone();
    nil[SETUP_OPERATION_ID_FIELD] = serde_json::json!(uuid::Uuid::nil());
    let mut private = valid.clone();
    private["credentials"] = serde_json::json!("not-public");
    let mut unknown = valid.clone();
    unknown[SETUP_HISTORY_CHECKPOINT_FIELD][SETUP_CHECKPOINT_PHASE_TAG] =
        serde_json::json!("running");
    let mut nested = valid;
    nested[SETUP_HISTORY_CHECKPOINT_FIELD]["request"] = serde_json::json!({});
    for malformed in [nil, private, unknown, nested] {
        assert!(serde_json::from_value::<RemoteSetupHistory>(malformed).is_err());
    }
}
