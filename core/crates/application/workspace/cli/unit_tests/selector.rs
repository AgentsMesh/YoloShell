use super::Matches;
use yolo_hub_identity::{HubId, WorkspaceAddress, WorkspaceId};

#[test]
fn later_origin_with_exact_name_wins_over_a_folded_match() {
    let first = WorkspaceAddress {
        hub_id: HubId::new(),
        workspace_id: WorkspaceId::new(),
    };
    let second = WorkspaceAddress {
        hub_id: HubId::new(),
        workspace_id: WorkspaceId::new(),
    };
    let mut matches = Matches::default();
    matches.consider(first, "équipe", "Équipe", "équipe", None);
    matches.consider(second, "Équipe", "Équipe", "équipe", None);
    assert_eq!(matches.resolve().unwrap(), second);

    for (selector, name) in [("ÉQUIPE", "équipe"), ("Équipe", "Équipe")] {
        let mut matches = Matches::default();
        matches.consider(first, name, selector, "équipe", None);
        matches.consider(second, name, selector, "équipe", None);
        assert!(matches.resolve().is_err());
    }
    let id = Some(second.workspace_id.as_uuid());
    let mut matches = Matches::default();
    matches.consider(first, "équipe", "", "", id);
    matches.consider(second, "Équipe", "", "", id);
    assert_eq!(matches.resolve().unwrap(), second);
}
