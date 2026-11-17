use crate::support::{constants::*, *};
use yolo_hub_routing::{Grant, Right, Rights, RoutingError};

#[test]
fn independent_rights_never_imply_other_permissions() {
    let all = [
        Right::Observe,
        Right::TerminalRead,
        Right::TerminalWrite,
        Right::Lifecycle,
        Right::FileRead,
        Right::AgentControl,
    ];
    for owned in all {
        let granted: Rights = [owned].into_iter().collect();
        for needed in all {
            let required: Rights = [needed].into_iter().collect();
            assert_eq!(granted.allows(&required), owned == needed);
            assert_eq!(granted.intersection(&required).is_empty(), owned != needed);
        }
    }
    assert!(Rights::default().is_empty());
    assert!(rights().allows(&Rights::default()));
    assert!(!Rights::default().allows(&rights()));
}

#[test]
fn grants_attenuate_both_rights_and_reexports() {
    let ceiling = Grant {
        rights: observe(),
        remaining_reexports: SINGLE_REEXPORT,
    };
    assert_eq!(grant().attenuate(&ceiling), ceiling);
    assert_eq!(ceiling.attenuate(&grant()), ceiling);
    let denied = Grant {
        rights: Rights::default(),
        remaining_reexports: NO_REEXPORTS,
    };
    assert_eq!(grant().attenuate(&denied), denied);
}

#[test]
fn rights_roundtrip_is_strict_and_deterministic() {
    let encoded = serde_json::to_string(&rights()).unwrap();
    assert_eq!(serde_json::from_str::<Rights>(&encoded).unwrap(), rights());
    assert!(serde_json::from_str::<Rights>(UNKNOWN_RIGHT_JSON).is_err());
    let duplicated: Rights = [Right::Observe, Right::Observe].into_iter().collect();
    assert_eq!(duplicated, observe());
    let error: &dyn std::error::Error = &RoutingError::EmptyRights;
    assert!(error.to_string().starts_with(ERROR_PREFIX));
}
