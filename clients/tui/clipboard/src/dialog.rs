use crate::files::{ReferenceStyle, file_path};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{Frame, layout::Rect, widgets::Paragraph};
use std::path::PathBuf;
use yolo_hub_identity::RuntimeTarget;
use yolo_tui_line_input::Cursor;

#[derive(Clone, Copy, Debug)]
pub enum InitialSource {
    Clipboard,
    LocalFile,
}

pub(super) enum Source {
    Clipboard,
    File(PathBuf),
}

enum Step {
    Editing { path: String, cursor: Cursor },
    Ready(Source),
    Uploading,
}

pub struct Dialog {
    pub(super) target: RuntimeTarget,
    pub(super) style: ReferenceStyle,
    step: Step,
}

impl Dialog {
    pub fn new(target: RuntimeTarget, source: InitialSource, style: ReferenceStyle) -> Self {
        let step = match source {
            InitialSource::Clipboard => Step::Ready(Source::Clipboard),
            InitialSource::LocalFile => Step::Editing {
                path: String::new(),
                cursor: Cursor::default(),
            },
        };
        Self {
            target,
            step,
            style,
        }
    }

    /// Return true when Escape dismisses the request. Plain paste always edits the path.
    pub fn dismiss(&mut self, event: &Event) -> bool {
        let key = match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => Some(key.code),
            _ => None,
        };
        if key == Some(KeyCode::Esc) {
            return true;
        }
        if let Step::Editing { path, cursor } = &mut self.step {
            if key == Some(KeyCode::Enter) {
                if let Ok(path) = file_path(path) {
                    self.step = Step::Ready(Source::File(path));
                }
            } else {
                cursor.handle(path, event, usize::MAX);
            }
        }
        false
    }

    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect) {
        if let Step::Editing { path, cursor } = &self.step {
            yolo_tui_line_input::render(
                frame,
                area,
                "Attach file: ",
                " · Enter attaches · Esc cancels",
                path,
                cursor,
            );
        } else {
            frame.render_widget(Paragraph::new("Attaching… · Esc cancels"), area);
        }
    }

    pub(super) fn take_source(&mut self) -> Option<Source> {
        if !matches!(self.step, Step::Ready(_)) {
            return None;
        }
        match std::mem::replace(&mut self.step, Step::Uploading) {
            Step::Ready(source) => Some(source),
            _ => unreachable!(),
        }
    }
}
