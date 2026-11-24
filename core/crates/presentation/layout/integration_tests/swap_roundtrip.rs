use crate::{editing_fixture::*, fixture::roundtrip};
use yolo_presentation_layout::{Layout, SwapDirection};

#[test]
fn swapped_layout_persists_while_zoom_projection_never_overwrites_the_owner() {
    let mut layout = grid();
    let panes = layout.panes().into_iter().cloned().collect::<Vec<_>>();
    layout.swap_focused(SwapDirection::Previous).unwrap();
    let zoom = layout.focused_view();
    let encoded = serde_json::to_vec(&layout).unwrap();
    let restored: Layout = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, layout);
    assert_ne!(restored, zoom);
    assert_eq!(restored.panes().len(), panes.len());
    assert_eq!(
        restored.panes().last().unwrap().id,
        panes[usize::default()].id
    );
    roundtrip(&zoom);
}
