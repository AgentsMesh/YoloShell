use crate::{
    fields::{FieldId, Value},
    form::Form,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use yolo_hub_delegation::MAX_DELEGATION_DEPTH;
use yolo_hub_federation_settings::MAX_HOST_BYTES;

impl Form {
    pub fn paste(&mut self, value: &str) -> bool {
        let Some(field) = self.fields.get_mut(self.focused) else {
            return false;
        };
        if field.read_only {
            return false;
        }
        let limit = text_limit(field.id);
        let Value::Text { text, cursor } = &mut field.value else {
            return false;
        };
        cursor.handle(text, &Event::Paste(value.to_owned()), limit)
    }
    pub fn key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab | KeyCode::Down => self.move_focus(true),
            KeyCode::BackTab | KeyCode::Up => self.move_focus(false),
            _ => {
                let Some(field) = self.fields.get_mut(self.focused) else {
                    return;
                };
                if field.read_only {
                    return;
                }
                match &mut field.value {
                    Value::Toggle(value) => {
                        if matches!(
                            key.code,
                            KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right | KeyCode::Enter
                        ) {
                            *value = !*value;
                        }
                    }
                    Value::Stepper(value) => match key.code {
                        KeyCode::Left | KeyCode::Char('-') => *value = value.saturating_sub(1),
                        KeyCode::Right | KeyCode::Char('+') => {
                            *value = value.saturating_add(1).min(MAX_DELEGATION_DEPTH)
                        }
                        KeyCode::Home => *value = 0,
                        KeyCode::End => *value = MAX_DELEGATION_DEPTH,
                        _ => {}
                    },
                    Value::Text { text, cursor } => {
                        cursor.handle(text, &Event::Key(key), text_limit(field.id));
                    }
                }
            }
        }
    }
}

fn text_limit(id: FieldId) -> usize {
    match id {
        FieldId::RootName => yolo_hub_file_protocol::MAX_FILE_ROOT_NAME_BYTES,
        FieldId::RootPath => yolo_hub_file_protocol::MAX_FILE_PATH_BYTES,
        _ => MAX_HOST_BYTES,
    }
}
