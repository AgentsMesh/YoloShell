use super::fixture::*;
use crate::{
    constants::AGENT_CONTROL,
    fields::{FieldId, Value},
    form::Form,
    model::{Action, Page},
};
use crossterm::event::KeyCode;
use yolo_hub_routing::Right;

#[test]
fn agent_control_checkbox_defaults_off_and_each_permission_is_independent() {
    for (field, right) in [
        (FieldId::WriteTerminal, Right::TerminalWrite),
        (FieldId::FileRead, Right::FileRead),
        (FieldId::AgentControl, Right::AgentControl),
    ] {
        let mut manager = manager();
        let peer = peer_identity();
        add_peer(&mut manager, peer);
        manager.page(Page::Exports(peer.hub()));
        activate(&mut manager, Action::AddExport(peer.hub()));
        let form = manager.form.as_ref().unwrap();
        let control = form
            .fields
            .iter()
            .find(|field| field.id == FieldId::AgentControl)
            .unwrap();
        assert!(matches!(control.value, Value::Toggle(false)));
        assert!(!control.read_only);
        assert_eq!(control.label(), AGENT_CONTROL);
        let form = manager.form.as_mut().unwrap();
        form.focused = form
            .fields
            .iter()
            .position(|candidate| candidate.id == field)
            .unwrap();
        key(&mut manager, KeyCode::Char(' '));
        save(&mut manager);
        let policy = manager
            .draft
            .as_ref()
            .unwrap()
            .export(peer.hub(), manager.entry)
            .unwrap();
        assert_eq!(policy.rights, [Right::Observe, right].into_iter().collect());
    }
}

#[test]
fn editing_agent_control_roundtrips_form_without_dropping_terminal_or_file_rights() {
    let mut manager = manager();
    let peer = peer_identity();
    add_peer(&mut manager, peer);
    let local = manager.base.as_ref().unwrap().state.identity;
    let mut draft = manager.draft.clone().unwrap();
    draft.peers[0]
        .exports
        .push(yolo_hub_federation_settings::ExportPolicy {
            origin: local.hub(),
            rights: [
                Right::Observe,
                Right::TerminalWrite,
                Right::FileRead,
                Right::AgentControl,
            ]
            .into_iter()
            .collect(),
            remaining_reexports: yolo_hub_delegation::MAX_DELEGATION_DEPTH,
        });
    for enabled in [false, true] {
        let policy = draft.export(peer.hub(), local.hub()).unwrap();
        let mut form = Form::export(peer.hub(), local.hub(), Some(policy));
        let field = form
            .fields
            .iter_mut()
            .find(|field| field.id == FieldId::AgentControl)
            .unwrap();
        assert!(
            matches!(field.value, Value::Toggle(value) if value == policy.rights.allows(&[Right::AgentControl].into_iter().collect()))
        );
        field.value = Value::Toggle(enabled);
        draft = form.commit(&draft, local).unwrap();
        let rights = &draft.export(peer.hub(), local.hub()).unwrap().rights;
        assert_eq!(
            rights.allows(&[Right::AgentControl].into_iter().collect()),
            enabled
        );
        assert!(
            rights.allows(
                &[Right::TerminalWrite, Right::FileRead]
                    .into_iter()
                    .collect()
            )
        );
        assert_eq!(draft.peers[0].identity, peer);
    }
}

#[test]
fn agent_control_field_is_visible_in_the_export_editor() {
    let mut manager = manager();
    let peer = peer_identity();
    add_peer(&mut manager, peer);
    manager.page(Page::Exports(peer.hub()));
    activate(&mut manager, Action::AddExport(peer.hub()));
    let form = manager.form.as_mut().unwrap();
    form.focused = form
        .fields
        .iter()
        .position(|field| field.id == FieldId::AgentControl)
        .unwrap();
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(
        EDITOR_WIDTH,
        EDITOR_HEIGHT,
    ))
    .unwrap();
    terminal
        .draw(|frame| {
            crate::render(frame, &manager, frame.area());
        })
        .unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains(AGENT_CONTROL));
}

const EDITOR_WIDTH: u16 = 80;
const EDITOR_HEIGHT: u16 = 24;
