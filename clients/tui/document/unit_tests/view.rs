use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use yolo_tui_document::{View, text};
use yolo_tui_tab::{LocalTabView, TabCatalog};

fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new_with_kind(
        code,
        KeyModifiers::NONE,
        KeyEventKind::Press,
    ))
}

#[test]
fn opens_and_closes_the_layout_pane() {
    let mut tabs = TabCatalog::single(LocalTabView::default());
    let mut view = View::default();
    assert!(!view.is_open());
    assert!(view.open("report".into(), "hello".into(), &mut tabs));
    assert!(view.is_open());
    assert_eq!(tabs.active_document().unwrap().text, "hello");

    assert!(view.handle(&press(KeyCode::Esc)));
    view.close(&mut tabs);
    assert!(!view.is_open());
    assert!(tabs.active_document().is_none());

    assert!(tabs.open_document("restored".into(), "body".into()));
    let mut restored = View::default();
    assert!(restored.restore(&tabs));
    assert!(restored.is_open());
}

#[test]
fn reads_document_text_from_the_action_output() {
    assert_eq!(
        text(&serde_json::json!({"text": "report"})),
        Some("report")
    );
    assert_eq!(text(&serde_json::json!({})), None);
}
