use crate::constants::{ID, NIL};
use std::collections::HashSet;
use yolo_hub_identity::*;

macro_rules! identity_contract {
    ($test:ident, $id:ty) => {
        #[test]
        fn $test() {
            let parsed = ID.parse::<$id>().unwrap();
            let uuid = uuid::Uuid::parse_str(ID).unwrap();
            assert_eq!(parsed.as_uuid(), uuid);
            assert_eq!(<$id>::from_uuid(uuid).unwrap(), parsed);
            assert_eq!(String::from(parsed), ID);
            assert_eq!(<$id>::try_from(ID.to_owned()).unwrap(), parsed);
            assert_eq!(serde_json::to_value(parsed).unwrap(), ID);
            assert_eq!(
                serde_json::from_value::<$id>(serde_json::json!(ID)).unwrap(),
                parsed
            );
            assert_ne!(<$id>::default(), <$id>::new());
            assert_ne!(<$id>::new().as_uuid(), uuid::Uuid::nil());
            assert!(format!("{parsed:?}").contains(ID));
            let values = HashSet::from([parsed, parsed]);
            assert_eq!(values.len(), 1);
            assert_eq!(parsed.cmp(&parsed), std::cmp::Ordering::Equal);
            for invalid in [
                String::new(),
                NIL.into(),
                ID.to_uppercase(),
                ID.replace('-', ""),
                format!(" {ID}"),
                format!("{{{ID}}}"),
            ] {
                assert!(invalid.parse::<$id>().is_err(), "{invalid}");
                assert!(serde_json::from_value::<$id>(serde_json::json!(invalid)).is_err());
            }
            assert!(<$id>::from_uuid(uuid::Uuid::nil()).is_err());
            assert!(serde_json::from_value::<$id>(serde_json::Value::Null).is_err());
        }
    };
}

identity_contract!(hub_identity_is_canonical_and_non_nil, HubId);
identity_contract!(session_identity_is_canonical_and_non_nil, TerminalId);
identity_contract!(agent_identity_is_canonical_and_non_nil, AgentId);
identity_contract!(runtime_epoch_is_canonical_and_non_nil, RuntimeEpoch);
identity_contract!(agent_execution_is_canonical_and_non_nil, ExecutionId);
identity_contract!(file_root_identity_is_canonical_and_non_nil, FileRootId);
identity_contract!(file_root_epoch_is_canonical_and_non_nil, FileRootEpoch);
identity_contract!(connector_identity_is_canonical_and_non_nil, ConnectorId);
identity_contract!(
    connector_generation_is_canonical_and_non_nil,
    ConnectorGeneration
);
identity_contract!(
    connector_instance_is_canonical_and_non_nil,
    ConnectorInstanceId
);

#[test]
fn identity_error_is_an_actionable_stable_error() {
    let error = NIL.parse::<HubId>().unwrap_err();
    assert!(!error.to_string().is_empty());
    assert!(std::error::Error::source(&error).is_none());
}
