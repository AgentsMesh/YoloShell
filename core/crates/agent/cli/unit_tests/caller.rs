use crate::caller::{check_hub, parse_workspace};
use uuid::Uuid;

#[test]
fn absent_or_blank_workspace_is_not_inherited() {
    assert_eq!(parse_workspace(None).unwrap(), None);
    assert_eq!(parse_workspace(Some("")).unwrap(), None);
    assert_eq!(parse_workspace(Some("   ")).unwrap(), None);
}

#[test]
fn inherited_workspace_must_be_a_uuid() {
    let workspace = Uuid::from_u128(7);
    assert_eq!(
        parse_workspace(Some(&workspace.to_string())).unwrap(),
        Some(workspace)
    );
    assert!(parse_workspace(Some("not-a-workspace")).is_err());
    assert!(parse_workspace(Some(&Uuid::nil().to_string())).is_err());
}

#[test]
fn inherited_hub_must_match_the_connected_hub() {
    let hub = Uuid::from_u128(11);
    assert!(check_hub(Some(&hub.to_string()), hub).is_ok());
    assert!(check_hub(Some(&Uuid::from_u128(12).to_string()), hub).is_err());
    assert!(check_hub(Some("garbage"), hub).is_err());
    assert!(check_hub(None, hub).is_ok());
    assert!(check_hub(Some("  "), hub).is_ok());
}
