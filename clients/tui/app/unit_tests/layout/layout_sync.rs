use super::layout_support::*;
use crate::layout_sync::{complete, pump, refresh};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummary};
use yolo_presentation_layout::{Axis, Layout};

#[test]
fn client_focus_and_zoom_do_not_publish_shared_mutations() {
    let (mut state, jobs, mut incoming) = fixture();
    state.toggle_pane_zoom();
    pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    state
        .tab_layout_mut()
        .split(Axis::Horizontal, None)
        .unwrap();
    let operation = request(&mut state, &jobs, &mut incoming);
    let accepted = applied(&state, &operation);
    response(&mut state, operation, accepted);
    let first = state.tab_layout().panes()[0].id;
    state.tab_layout_mut().focus(first).unwrap();
    state.mode = crate::Mode::Terminal;
    state.toggle_pane_zoom();
    pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
}

#[test]
fn accepted_write_keeps_edits_made_while_the_reply_was_in_flight() {
    let (mut state, jobs, mut incoming) = fixture();
    state
        .tab_layout_mut()
        .split(Axis::Horizontal, None)
        .unwrap();
    let operation = request(&mut state, &jobs, &mut incoming);
    let accepted = applied(&state, &operation);
    state.tab_layout_mut().split(Axis::Vertical, None).unwrap();
    let draft = state.tab_layout().clone();
    response(&mut state, operation, accepted.clone());
    assert_eq!(*state.tab_layout(), draft);
    let next = request(&mut state, &jobs, &mut incoming);
    let Command::UpdateLayout { change } = next.command else {
        panic!()
    };
    assert_eq!(change.expected_revision, accepted.revision);
    assert_eq!(change.tabs[0].root.panes().len(), draft.panes().len());
}

#[test]
fn uncertain_write_reads_original_identity_before_further_updates() {
    let (mut state, jobs, mut incoming) = fixture();
    state.tab_layout_mut().split(Axis::Vertical, None).unwrap();
    let operation = request(&mut state, &jobs, &mut incoming);
    let accepted = applied(&state, &operation);
    complete(&mut state, operation, Err("lost reply".into()));
    assert!(state.shared_layout.as_ref().unwrap().uncertain.is_some());
    pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    state.shared_layout.as_mut().unwrap().retry = std::time::Instant::now();
    let read = request(&mut state, &jobs, &mut incoming);
    assert!(matches!(read.command, Command::GetLayout { .. }));
    response(&mut state, read, accepted.clone());
    assert_eq!(state.shared_layout.as_ref().unwrap().document, accepted);
    assert!(state.shared_layout.as_ref().unwrap().uncertain.is_none());
    pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
}

#[test]
fn concurrent_remote_change_loads_authoritative_revision_without_terminating_shells() {
    let (mut state, jobs, mut incoming) = fixture();
    let mut other = remote(&state);
    other.tabs[0].root = Layout::default().document().root.clone();
    refresh(&mut state);
    let read = request(&mut state, &jobs, &mut incoming);
    state
        .tab_layout_mut()
        .split(Axis::Horizontal, None)
        .unwrap();
    response(&mut state, read, other.clone());
    assert_eq!(state.tab_snapshot().shared_tabs(state.entry_hub).unwrap(), other.tabs);
    assert!(state.status.contains("Terminals remain running"));
    pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
}

#[test]
fn remote_empty_document_is_not_immediately_repopulated_by_placeholder() {
    let (mut state, jobs, mut incoming) = fixture();
    let mut other = remote(&state);
    other.tabs.clear();
    refresh(&mut state);
    let read = request(&mut state, &jobs, &mut incoming);
    response(&mut state, read, other);
    pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    assert!(
        state
            .shared_layout
            .as_ref()
            .unwrap()
            .document
            .tabs
            .is_empty()
    );
}

#[test]
fn adopted_layout_cannot_bind_a_gone_terminal_and_publishes_the_prune() {
    let (mut state, jobs, mut incoming) = fixture();
    let mut fields = super::fixtures::session().fields().clone();
    fields.runtime.as_mut().unwrap().status = RuntimeStatus::Exited;
    let ended = TerminalSummary::new(fields).unwrap();
    let address = ended.fields().address;
    state.replace_terminals(vec![ended.clone()]);

    let mut other = remote(&state);
    other.tabs[0].root = Layout::new(Some(address)).document().root.clone();
    refresh(&mut state);
    let read = request(&mut state, &jobs, &mut incoming);
    response(&mut state, read, other);
    assert_eq!(state.focused_terminal(), None);
    let update = request(&mut state, &jobs, &mut incoming);
    let accepted = applied(&state, &update);
    assert!(
        accepted.tabs[0]
            .root
            .panes()
            .iter()
            .all(|pane| pane.terminal.is_none())
    );
}

#[test]
fn layout_platform_event_loads_the_authoritative_document() {
    let (mut state, jobs, mut incoming) = fixture();
    crate::events::apply(
        &mut state,
        &jobs,
        crate::network::Update::PlatformEvents(yolo_hub_client_channels::PlatformEventWake {
            lost: false,
            layout_changed: true,
        }),
    );
    assert!(state.shared_layout.as_ref().unwrap().read);
    assert!(incoming.try_recv().is_ok());
    assert!(matches!(
        request(&mut state, &jobs, &mut incoming).command,
        Command::GetLayout { .. }
    ));
}

#[test]
fn unrelated_platform_event_does_not_reload_the_layout() {
    let (mut state, jobs, _incoming) = fixture();
    crate::events::apply(
        &mut state,
        &jobs,
        crate::network::Update::PlatformEvents(yolo_hub_client_channels::PlatformEventWake {
            lost: false,
            layout_changed: false,
        }),
    );
    assert!(!state.shared_layout.as_ref().unwrap().read);
}
