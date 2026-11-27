use super::layout::{reply, request};
use uuid::Uuid;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{Command, Reply, Request};
use yolo_presentation_layout::Layout;
use yolo_presentation_protocol::LayoutUpdate;
use yolo_tui_tab::TabStore;

#[test]
fn initial_layout_is_allowed_but_foreign_layout_and_terminal_binding_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let hub = HubId::new();
    let store = TabStore::open(root.path(), hub).unwrap();
    let change = LayoutUpdate {
        address: store.document().layout_address,
        request_id: Uuid::new_v4(),
        expected_revision: u64::default(),
        tabs: store.document().tabs.shared_tabs().unwrap(),
    };
    let original = Request::new(
        hub,
        Command::UpdateLayout {
            change: change.clone(),
        },
    )
    .unwrap();
    assert!(request(&original, root.path()).is_ok());
    let document = change.apply(None).unwrap();
    assert!(
        reply(
            &original,
            &Reply::Layout {
                document: Some(document.clone())
            }
        )
        .is_ok()
    );
    let mut foreign = change.clone();
    foreign.address.layout_id = Uuid::new_v4();
    assert!(
        request(
            &Request::new(hub, Command::UpdateLayout { change: foreign }).unwrap(),
            root.path()
        )
        .is_err()
    );
    let mut bound = change;
    bound.tabs.first_mut().unwrap().root = Layout::new(Some(TerminalAddress {
        hub_id: hub,
        terminal_id: TerminalId::new(),
    }))
    .document()
    .root
    .clone();
    assert!(
        request(
            &Request::new(hub, Command::UpdateLayout { change: bound }).unwrap(),
            root.path()
        )
        .is_err()
    );
    assert!(reply(&original, &Reply::Completed {}).is_err());
    let mut mismatched = document;
    mismatched.last_request_id = Uuid::new_v4();
    assert!(
        reply(
            &original,
            &Reply::Layout {
                document: Some(mismatched)
            }
        )
        .is_err()
    );
}
