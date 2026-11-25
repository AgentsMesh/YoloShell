use crate::{editing_fixture::*, fixture::roundtrip};
use yolo_presentation_layout::{Layout, PaneRect};

#[test]
fn focused_projection_retains_identity_binding_and_entire_owner_layout() {
    let mut layout = grid();
    for focused in [TOP_LEFT, TOP_RIGHT, BOTTOM_LEFT, BOTTOM_RIGHT] {
        layout.focus(id(focused)).unwrap();
        let before = layout.clone();
        let view = layout.focused_view();
        assert_eq!(view.focused(), layout.focused());
        assert_eq!(
            view.panes(),
            vec![
                layout
                    .panes()
                    .into_iter()
                    .find(|pane| pane.id == id(focused))
                    .unwrap()
            ]
        );
        assert_eq!(
            view.geometry(bounds()).unwrap(),
            vec![PaneRect {
                pane: id(focused),
                rect: bounds()
            }]
        );
        assert_eq!(layout, before);
        roundtrip(&view);
    }
}

#[test]
fn empty_single_pane_projection_keeps_the_original_identifier() {
    let layout = Layout::default();
    assert_eq!(layout.focused_view(), layout);
}
