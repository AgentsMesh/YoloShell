use crate::{Action, Form, Manager, constants::*};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use yolo_tui_tab::{MAX_TAB_NAME_BYTES, TabCatalog};

impl Manager {
    pub fn handle(&mut self, event: Event, tabs: &TabCatalog) -> Option<Action> {
        if let Event::Paste(_) = &event {
            if matches!(self.form, Some(Form::Create | Form::Rename(_))) {
                self.cursor
                    .handle(&mut self.text, &event, MAX_TAB_NAME_BYTES);
            }
            return None;
        }
        let Event::Key(key) = event else {
            return None;
        };
        if key.kind != KeyEventKind::Press {
            return None;
        }
        let plain = key.modifiers.difference(KeyModifiers::SHIFT).is_empty();
        if key.code == KeyCode::Esc && plain {
            if self.form.take().is_some() {
                self.message.clear();
                return None;
            }
            return Some(Action::Close);
        }
        if let Some(form) = self.form.clone() {
            return match key.code {
                KeyCode::Enter if plain => Some(match form {
                    Form::Create => Action::Create(self.text.trim().into()),
                    Form::Rename(id) => Action::Rename(id, self.text.trim().into()),
                    Form::Remove(id) => Action::Remove(id),
                }),
                _ if !matches!(form, Form::Remove(_)) => {
                    self.cursor
                        .handle(&mut self.text, &Event::Key(key), MAX_TAB_NAME_BYTES);
                    None
                }
                _ => None,
            };
        }
        if !plain {
            return None;
        }
        match key.code {
            KeyCode::Char(CLOSE_KEY) => return Some(Action::Close),
            KeyCode::Down | KeyCode::Char(NEXT_KEY) => self.move_selection(tabs, true),
            KeyCode::Up | KeyCode::Char(PREVIOUS_KEY) => self.move_selection(tabs, false),
            KeyCode::Enter => return Some(Action::Select(self.selected)),
            KeyCode::Char(CREATE_KEY) => self.create(),
            KeyCode::Char(RENAME_KEY) => self.rename(tabs),
            KeyCode::Char(REMOVE_KEY) => self.form = Some(Form::Remove(self.selected)),
            _ => {}
        }
        None
    }

    fn move_selection(&mut self, tabs: &TabCatalog, forward: bool) {
        let entries = tabs.machine_entries();
        let index = entries
            .iter()
            .position(|entry| entry.id == self.selected)
            .unwrap_or_default();
        let next = if forward {
            (index + SELECTION_STEP).min(entries.len() - SELECTION_STEP)
        } else {
            index.saturating_sub(SELECTION_STEP)
        };
        self.selected = entries[next].id;
    }
}
