use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{Frame, layout::Rect};
use uuid::Uuid;
use yolo_tui_render::render_document;
use yolo_tui_tab::TabCatalog;

/// The visible document pane, independent of the plugin invocation that
/// produced it. The document belongs to one local tab, not the shared layout.
#[derive(Default)]
pub struct View {
    tab: Option<Uuid>,
    scroll: u16,
}

impl View {
    pub fn is_open(&self) -> bool {
        self.tab.is_some()
    }

    pub fn is_visible(&self, tabs: &TabCatalog) -> bool {
        self.is_open() && self.tab == Some(tabs.active().id) && tabs.active_document().is_some()
    }

    pub fn open(&mut self, title: String, text: String, tabs: &mut TabCatalog) -> bool {
        self.tab = Some(tabs.active().id);
        self.scroll = 0;
        tabs.open_document(title, text)
    }

    /// Re-arms the view from a persisted tab document.
    pub fn restore(&mut self, tabs: &TabCatalog) -> bool {
        self.tab = tabs
            .active_document()
            .map(|_| tabs.active().id);
        self.scroll = 0;
        self.is_open()
    }

    pub fn close(&mut self, tabs: &mut TabCatalog) {
        if self.is_open() {
            tabs.close_document();
        }
        self.tab = None;
        self.scroll = 0;
    }

    /// Consumes every event while open; returns whether the view should close.
    pub fn handle(&mut self, event: &Event) -> bool {
        let Event::Key(key) = event else {
            return false;
        };
        if key.kind != KeyEventKind::Press {
            return false;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1)
            }
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
            KeyCode::Home => self.scroll = 0,
            _ => {}
        }
        false
    }

    pub fn render(&self, frame: &mut Frame<'_>, tabs: &TabCatalog, area: Rect) {
        let Some(document) = tabs.active_document() else {
            return;
        };
        render_document(frame, &document.title, &document.text, self.scroll, area);
    }
}

/// Reads the bounded ANSI text a document action returned.
pub fn text(output: &serde_json::Value) -> Option<&str> {
    output.get("text").and_then(serde_json::Value::as_str)
}

/// Opens the returned text as a pane, preserving the action's error surface.
pub fn open(
    view: &mut View,
    tabs: &mut TabCatalog,
    title: String,
    output: &serde_json::Value,
) -> Result<(), &'static str> {
    let Some(text) = text(output) else {
        return Err(crate::INVALID_DOCUMENT);
    };
    if view.open(title, text.to_owned(), tabs) {
        Ok(())
    } else {
        Err(crate::DOCUMENT_PANE_FAILED)
    }
}
