use super::{fixture::*, *};
use uuid::Uuid;

#[test]
fn summary_counts_topology_without_transmitting_bindings_or_private_client_state() {
    let mut request = update();
    request.tabs.push(tab());
    let document = request.apply(None).unwrap();
    let summary = LayoutSummary::try_from(&document).unwrap();
    assert_eq!(summary.address, document.address);
    assert_eq!(summary.revision, document.revision);
    assert_eq!(summary.tab_count as usize, document.tabs.len());
    assert_eq!(summary.pane_count as usize, document.tabs.len());
    let wire = serde_json::to_value(summary).unwrap();
    assert_eq!(wire.as_object().unwrap().len(), SUMMARY_FIELDS);
    assert_eq!(
        serde_json::from_value::<LayoutSummary>(wire).unwrap(),
        summary
    );
    let mut invalid = document;
    invalid.revision = 0;
    assert_eq!(
        LayoutSummary::try_from(&invalid),
        Err(ProtocolError::InvalidRevision)
    );
}

#[test]
fn summaries_validate_empty_and_maximum_layouts_and_reject_impossible_counts() {
    let mut summary = LayoutSummary::try_from(&update().apply(None).unwrap()).unwrap();
    for (tabs, panes) in [(0, 0), (MAX_TABS, MAX_DOCUMENT_PANES)] {
        summary.tab_count = tabs as u32;
        summary.pane_count = panes as u32;
        assert!(summary.validate().is_ok());
    }
    for (tabs, panes) in [
        (MAX_TABS + 1, MAX_TABS + 1),
        (MAX_TABS, MAX_DOCUMENT_PANES + 1),
        (1, 0),
        (0, 1),
        (1, yolo_presentation_layout::MAX_PANES + 1),
    ] {
        summary.tab_count = tabs as u32;
        summary.pane_count = panes as u32;
        assert_eq!(summary.validate(), Err(ProtocolError::Limit));
        assert!(
            serde_json::from_value::<LayoutSummary>(serde_json::to_value(summary).unwrap())
                .is_err()
        );
    }
    summary.address.layout_id = Uuid::nil();
    assert_eq!(summary.validate(), Err(ProtocolError::InvalidId));
    summary.address.layout_id = Uuid::new_v4();
    summary.revision = 0;
    assert_eq!(summary.validate(), Err(ProtocolError::InvalidRevision));
}

#[test]
fn summary_wire_rejects_unknown_fields() {
    let summary = LayoutSummary::try_from(&update().apply(None).unwrap()).unwrap();
    let mut wire = serde_json::to_value(summary).unwrap();
    wire[FOREIGN_FIELD] = true.into();
    assert!(serde_json::from_value::<LayoutSummary>(wire).is_err());
}
