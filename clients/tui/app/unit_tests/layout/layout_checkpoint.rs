use super::layout_support::*;
use crate::layout_sync::startup::restore_draft;
use yolo_presentation_layout::Axis;
use yolo_tui_tab::LayoutCheckpoint;

#[test]
fn restart_preserves_unsent_draft_if_the_published_base_is_unchanged() {
    let (mut state, _, _) = fixture();
    let sync = state.shared_layout.as_ref().unwrap();
    let checkpoint = LayoutCheckpoint {
        document: sync.document.clone(),
        projected: sync.projected.clone(),
    };
    state
        .tab_layout_mut()
        .split(Axis::Horizontal, None)
        .unwrap();
    let draft = state.tab_layout().clone();
    let projected = restore_draft(&mut state, &checkpoint.document, Some(&checkpoint)).unwrap();
    assert_eq!(*state.tab_layout(), draft);
    assert_eq!(projected, checkpoint.projected);
}

#[test]
fn restart_accepts_matching_committed_draft_and_retains_conflicting_draft() {
    let (mut state, _, _) = fixture();
    let sync = state.shared_layout.as_ref().unwrap();
    let checkpoint = LayoutCheckpoint {
        document: sync.document.clone(),
        projected: sync.projected.clone(),
    };
    state
        .tab_layout_mut()
        .split(Axis::Horizontal, None)
        .unwrap();
    let draft = state.tab_layout().clone();
    let other = remote(&state);
    assert!(restore_draft(&mut state, &other, Some(&checkpoint)).is_err());
    assert_eq!(*state.tab_layout(), draft);
    let mut accepted = other;
    accepted.tabs = state.tab_snapshot().shared_tabs(state.entry_hub).unwrap();
    assert_eq!(
        restore_draft(&mut state, &accepted, Some(&checkpoint)).unwrap(),
        accepted.tabs
    );
    assert_eq!(*state.tab_layout(), draft);
}

#[test]
fn restart_without_local_edits_loads_new_published_topology() {
    let (mut state, _, _) = fixture();
    let sync = state.shared_layout.as_ref().unwrap();
    let checkpoint = LayoutCheckpoint {
        document: sync.document.clone(),
        projected: sync.projected.clone(),
    };
    let mut other = remote(&state);
    other.tabs[0].title = "renamed remotely".into();
    let projected = restore_draft(&mut state, &other, Some(&checkpoint)).unwrap();
    assert_eq!(projected, other.tabs);
    assert_eq!(state.tabs.active().name, other.tabs[0].title);
    assert_eq!(restore_draft(&mut state, &other, None).unwrap(), other.tabs);
}
