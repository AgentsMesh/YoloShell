use super::{Draft, Handle, Pending, Picker};
use crate::constants::*;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use std::path::Path;

impl Picker {
    pub fn key(&mut self, event: &Event) -> Handle {
        if !self.visible {
            return Handle::None;
        }
        if self.pending.is_some() {
            return Handle::None;
        }
        if let Some(draft) = self.draft.as_mut() {
            if !matches!(event, Event::Paste(_)) {
                if let Event::Key(key) = event {
                    if !matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
                        self.message = None;
                    }
                }
            }
            return match draft.handle(event) {
                super::draft::Action::Editing => Handle::None,
                super::draft::Action::Cancel => {
                    self.draft = None;
                    Handle::None
                }
                super::draft::Action::Invalid => {
                    self.message = Some(INVALID_NAME.into());
                    Handle::None
                }
                super::draft::Action::InvalidLabels => {
                    self.message = Some(INVALID_LABELS.into());
                    Handle::None
                }
                super::draft::Action::Submit(super::draft::Request::Create(request)) => {
                    self.pending = Some(Pending::Create(request.fields().workspace_id));
                    self.message = None;
                    Handle::Create(request)
                }
                super::draft::Action::Submit(super::draft::Request::Rename(request)) => {
                    let pending = match draft.kind() {
                        super::draft::Kind::Labels => Pending::Labels(request.fields().address),
                        _ => Pending::Rename(request.fields().address),
                    };
                    self.pending = Some(pending);
                    self.message = None;
                    Handle::Rename(request)
                }
            };
        }
        let Event::Key(key) = event else {
            return Handle::None;
        };
        if key.kind != KeyEventKind::Press {
            return Handle::None;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.close();
                Handle::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                Handle::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1).min(self.rows.len());
                Handle::None
            }
            KeyCode::Home => {
                self.selected = 0;
                Handle::None
            }
            KeyCode::End => {
                self.selected = self.rows.len();
                Handle::None
            }
            KeyCode::Char('l') if !self.loading && self.selected > 0 => {
                let row = self.rows[self.selected - 1].clone();
                self.message = None;
                self.draft = Some(Draft::labels(row.address, row.revision, &row.labels));
                Handle::None
            }
            KeyCode::Char('e') if !self.loading && self.selected > 0 => {
                let row = self.rows[self.selected - 1].clone();
                self.message = None;
                self.draft = Some(Draft::rename(
                    row.address,
                    row.revision,
                    row.name,
                ));
                Handle::None
            }
            KeyCode::Char('c') if !self.loading => {
                self.message = None;
                let root = self.root.clone().unwrap_or_default();
                let name = default_name(&root);
                self.draft = Some(Draft::create(root, name));
                Handle::None
            }
            KeyCode::Char('r') if !self.loading => {
                self.rows.clear();
                self.selected = 0;
                self.message = None;
                self.loading = true;
                self.list_in_flight = true;
                match super::list_request(None) {
                    Some(request) => Handle::Query(request),
                    None => Handle::None,
                }
            }
            KeyCode::Enter if !self.loading => self.filter(),
            _ => Handle::None,
        }
    }

    fn filter(&mut self) -> Handle {
        let workspace = if self.selected == 0 {
            None
        } else {
            self.rows.get(self.selected - 1).map(|row| row.address)
        };
        self.close();
        Handle::Filter(workspace)
    }
}

fn default_name(root: &str) -> String {
    Path::new(root)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(DEFAULT_WORKSPACE_NAME)
        .to_owned()
}
