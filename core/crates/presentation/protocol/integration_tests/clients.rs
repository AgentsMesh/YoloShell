use super::{fixture::*, *};
use uuid::Uuid;
use yolo_presentation_layout::{Axis, Layout, PaneLayoutDocument as ClientView};

#[test]
fn independent_clients_share_only_topology_and_resolve_revision_conflicts() {
    let request = update();
    let shared = request.apply(None).unwrap();
    let root = shared.tabs[0].root.clone();
    let first = root.panes()[0].id;
    let mut a = Layout::try_from(ClientView {
        root: root.clone(),
        focused: first,
    })
    .unwrap();
    let b = Layout::try_from(ClientView {
        root,
        focused: first,
    })
    .unwrap();
    let added = a.split(Axis::Horizontal, None).unwrap();
    let mut edit = LayoutUpdate {
        address: shared.address,
        request_id: Uuid::new_v4(),
        expected_revision: shared.revision,
        tabs: shared.tabs.clone(),
    };
    edit.tabs[0].root = a.document().root.clone();
    let updated = edit.apply(Some(&shared)).unwrap();
    assert_eq!(a.focused(), added);
    assert_eq!(b.focused(), first);
    assert_eq!(
        edit.apply(Some(&updated)),
        Err(ProtocolError::RevisionConflict)
    );
    let b = Layout::try_from(ClientView {
        root: updated.tabs[0].root.clone(),
        focused: b.focused(),
    })
    .unwrap();
    assert_eq!(b.focused(), first);
    assert_eq!(b.panes().len(), a.panes().len());
}
