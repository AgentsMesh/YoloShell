use super::fixture::*;
use crate::{
    Manager, View,
    fields::{FieldId, Value},
    form::Form,
    model::{Action, Page},
};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Terminal, backend::TestBackend};

fn view(manager: &Manager) -> View {
    let mut terminal = Terminal::new(TestBackend::new(80, 32)).unwrap();
    let mut view = None;
    terminal
        .draw(|frame| view = Some(crate::render(frame, manager, frame.area())))
        .unwrap();
    view.unwrap()
}
fn click(manager: &mut Manager, row: usize) -> Option<crate::Effect> {
    let view = view(manager);
    let (rect, _) = view.hits.iter().find(|(_, index)| *index == row).unwrap();
    manager.pointer(
        &view,
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x,
            row: rect.y,
            modifiers: KeyModifiers::NONE,
        },
    )
}

#[test]
fn every_form_and_action_row_renders_within_compact_and_wide_viewports() {
    let mut manager = manager();
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    let peer = manager
        .draft
        .as_ref()
        .unwrap()
        .peer(pin.hub())
        .unwrap()
        .clone();
    let policy = yolo_hub_federation_settings::ExportPolicy {
        origin: manager.entry,
        rights: [yolo_hub_routing::Right::Observe].into_iter().collect(),
        remaining_reexports: 0,
    };
    let forms = [
        Form::peer(None),
        Form::peer(Some(&peer)),
        Form::origin(),
        Form::listener(None),
        Form::listener(Some("127.0.0.1:0".parse().unwrap())),
        Form::export(pin.hub(), manager.entry, None),
        Form::export(pin.hub(), manager.entry, Some(&policy)),
        Form::file_root(None),
        Form::file_root(Some(&super::files_support::add(&mut manager))),
    ];
    for form in forms {
        manager.form = Some(form);
        for index in 0..manager.form.as_ref().unwrap().rows() {
            manager.form.as_mut().unwrap().focused = index;
            for (width, height) in [(80, 24), (40, 16), (1, 3), (0, 0)] {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal
                    .draw(|frame| {
                        let view = crate::render(frame, &manager, frame.area());
                        assert!(
                            view.hits
                                .iter()
                                .all(|(rect, _)| rect.right() <= width && rect.bottom() <= height)
                        );
                    })
                    .unwrap();
            }
        }
    }
}

#[test]
fn mouse_focus_checkbox_cancel_and_confirmation_have_scoped_actions() {
    let mut manager = manager();
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    manager.page(Page::Peer(pin.hub()));
    activate(&mut manager, Action::EditPeer(pin.hub()));
    let host = manager.form.as_ref().unwrap().focused;
    click(&mut manager, 0);
    assert_eq!(manager.form.as_ref().unwrap().focused, host);
    let import = manager
        .form
        .as_ref()
        .unwrap()
        .fields
        .iter()
        .position(|field| field.id == FieldId::Import)
        .unwrap();
    click(&mut manager, import);
    assert!(matches!(
        manager.form.as_ref().unwrap().fields[import].value,
        Value::Toggle(false)
    ));
    click(&mut manager, host);
    assert_eq!(manager.form.as_ref().unwrap().focused, host);
    let cancel = manager.form.as_ref().unwrap().fields.len() + 1;
    click(&mut manager, cancel);
    assert!(manager.form.is_none());
    manager.page(Page::Root);
    activate(&mut manager, Action::Close);
    assert!(click(&mut manager, 0).is_none());
    assert!(manager.confirmation.is_none());
    activate(&mut manager, Action::Close);
    assert!(matches!(click(&mut manager, 1), Some(crate::Effect::Close)));
}

#[test]
fn scroll_stale_outside_and_nonprimary_pointer_events_do_not_apply_configuration() {
    let mut manager = manager();
    let initial = view(&manager);
    let event = |kind| MouseEvent {
        kind,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    manager.pointer(&initial, event(MouseEventKind::ScrollDown));
    assert_eq!(manager.selected, 1);
    let current = view(&manager);
    manager.pointer(&current, event(MouseEventKind::ScrollUp));
    assert_eq!(manager.selected, 0);
    let current = view(&manager);
    for kind in [
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Up(MouseButton::Left),
        MouseEventKind::Moved,
        MouseEventKind::Down(MouseButton::Left),
    ] {
        assert!(manager.pointer(&current, event(kind)).is_none());
    }
    assert!(manager.pending.is_none());
    let request = manager.query(false);
    let pending = view(&manager);
    assert!(
        manager
            .pointer(&pending, event(MouseEventKind::ScrollDown))
            .is_none()
    );
    manager.not_sent(request.token);
    manager.form = Some(Form::export(peer_identity().hub(), manager.entry, None));
    let depth = manager
        .form
        .as_ref()
        .unwrap()
        .fields
        .iter()
        .position(|field| field.id == FieldId::Reexports)
        .unwrap();
    click(&mut manager, depth);
    assert_eq!(manager.form.as_ref().unwrap().focused, depth);
}

#[test]
fn loading_failure_all_pages_and_confirmation_are_rendered_without_fake_configuration() {
    let snapshot = snapshot();
    let (mut manager, request) = Manager::open(snapshot.state.identity.hub());
    view(&manager);
    manager.not_sent(request.token);
    view(&manager);
    let request = manager.query(false);
    manager.complete(request.token, Ok(snapshot));
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    manager.draft.as_mut().unwrap().trusted_origins.push(pin);
    for page in [
        Page::Root,
        Page::Peers,
        Page::Peer(pin.hub()),
        Page::Origins,
        Page::Exports(pin.hub()),
    ] {
        manager.page(page);
        view(&manager);
    }
    manager.page(Page::Origins);
    activate(&mut manager, Action::RemoveOrigin(pin.hub()));
    view(&manager);
    key(&mut manager, KeyCode::Tab);
    view(&manager);
    key(&mut manager, KeyCode::Enter);
    assert!(manager.draft.as_ref().unwrap().trusted_origins.is_empty());
}
