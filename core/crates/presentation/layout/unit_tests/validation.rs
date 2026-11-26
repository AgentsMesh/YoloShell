use crate::fixture::*;
use uuid::Uuid;
use yolo_presentation_layout::*;

#[test]
fn persisted_layout_rejects_duplicate_nil_and_foreign_focus_identities() {
    let id = Uuid::new_v4();
    let pane = Tree::Pane(Pane { id, terminal: None });
    let duplicate = PaneLayoutDocument {
        root: Tree::Split {
            axis: Axis::Vertical,
            ratio: SplitRatio::default(),
            first: Box::new(pane.clone()),
            second: Box::new(pane.clone()),
        },
        focused: id,
    };
    assert_eq!(
        Layout::try_from(duplicate.clone()),
        Err(LayoutError::InvalidId)
    );
    assert!(serde_json::from_value::<Layout>(serde_json::to_value(duplicate).unwrap()).is_err());
    let missing = PaneLayoutDocument {
        root: pane,
        focused: Uuid::new_v4(),
    };
    assert_eq!(Layout::try_from(missing), Err(LayoutError::UnknownPane));
    let nil = PaneLayoutDocument {
        root: Tree::Pane(Pane {
            id: Uuid::nil(),
            terminal: None,
        }),
        focused: Uuid::nil(),
    };
    assert_eq!(Layout::try_from(nil), Err(LayoutError::InvalidId));
    assert_eq!(
        Layout::try_from(workspace(MAX_PANES + 1)),
        Err(LayoutError::Limit)
    );
}

#[test]
fn split_ratio_wire_is_bounded_and_does_not_silently_normalize_invalid_values() {
    for invalid in [0, 1, 99, 901, RATIO_SCALE, u16::MAX] {
        assert_eq!(
            SplitRatio::try_from(invalid),
            Err(LayoutError::InvalidRatio)
        );
        assert!(serde_json::from_value::<SplitRatio>(invalid.into()).is_err());
    }
    for valid in [100, 300, 500, 700, 900] {
        let ratio = SplitRatio::try_from(valid).unwrap();
        assert_eq!(u16::from(ratio), valid);
        assert_eq!(
            serde_json::from_value::<SplitRatio>(valid.into()).unwrap(),
            ratio
        );
        assert_eq!(serde_json::to_value(ratio).unwrap(), valid);
    }
    assert_eq!(u16::from(SplitRatio::default()), RATIO_SCALE / 2);
}

#[test]
fn unknown_document_and_split_fields_are_rejected() {
    let original = serde_json::to_value(workspace(2)).unwrap();
    for path in ["", "/root/split", "/root/split/first/pane"] {
        let mut document = original.clone();
        document
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), true.into());
        assert!(
            serde_json::from_value::<Layout>(document).is_err(),
            "{path}"
        );
    }
}

#[test]
fn errors_have_readable_messages_without_nested_sources() {
    for error in [
        LayoutError::UnknownPane,
        LayoutError::LastPane,
        LayoutError::InvalidId,
        LayoutError::InvalidRatio,
        LayoutError::Limit,
        LayoutError::InvalidRect,
        LayoutError::InvalidSplitPath,
    ] {
        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(&error).is_none());
    }
}
