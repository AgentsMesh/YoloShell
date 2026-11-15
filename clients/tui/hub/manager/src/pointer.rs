use crate::{Effect, Manager, View, fields::Value};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

impl Manager {
    pub fn pointer(&mut self, view: &View, event: MouseEvent) -> Option<Effect> {
        if view.epoch != self.epoch || self.pending() {
            return None;
        }
        match event.kind {
            MouseEventKind::ScrollUp => {
                return self.handle(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
            }
            MouseEventKind::ScrollDown => {
                return self.handle(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)));
            }
            MouseEventKind::Down(MouseButton::Left) => {}
            _ => return None,
        }
        let (_, index) = view
            .hits
            .iter()
            .find(|(bounds, _)| bounds.contains((event.column, event.row).into()))?;
        if let Some(confirmation) = &mut self.confirmation {
            confirmation.confirm = *index != 0;
        } else if let Some(form) = &mut self.form {
            if form.fields.get(*index).is_some_and(|field| field.read_only) {
                return None;
            }
            form.focused = *index;
            if let Some(field) = form.fields.get(*index) {
                if matches!(field.value, Value::Text { .. } | Value::Stepper(_)) {
                    self.epoch = uuid::Uuid::new_v4();
                    return None;
                }
            }
        } else {
            self.selected = *index;
        }
        self.handle(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
    }
}
