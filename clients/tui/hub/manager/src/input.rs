use crate::{
    Effect, Manager,
    constants::*,
    fields::Value,
    form::Form,
    model::{Action, Confirmation, Page},
};
use crossterm::event::{Event, KeyCode, KeyEventKind};

impl Manager {
    pub fn handle(&mut self, event: Event) -> Option<Effect> {
        self.epoch = uuid::Uuid::new_v4();
        if let Event::Paste(value) = &event {
            if let Some(form) = &mut self.form {
                if !form.paste(value) {
                    self.status = INPUT_REJECTED.into();
                }
            }
            return None;
        }
        let Event::Key(key) = event else {
            return None;
        };
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if self.pending.is_some() {
            if key.code == KeyCode::Esc
                && self
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.consumers)
            {
                self.pending = None;
                self.epoch = uuid::Uuid::new_v4();
                self.status = READY.into();
                return None;
            }
            return (key.code == KeyCode::Esc).then_some(Effect::Close);
        }
        if let Some(confirmation) = &mut self.confirmation {
            match key.code {
                KeyCode::Esc => self.confirmation = None,
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Down => {
                    confirmation.confirm = !confirmation.confirm
                }
                KeyCode::Enter => {
                    let confirmation = self.confirmation.take().unwrap();
                    if confirmation.confirm {
                        return self.perform(confirmation.action);
                    }
                }
                _ => {}
            }
            return None;
        }
        if let Some(form) = &mut self.form {
            if key.code == KeyCode::Esc {
                self.form = None;
                return None;
            }
            if key.code == KeyCode::Enter {
                if form.focused == form.fields.len() {
                    self.save_form();
                    return None;
                }
                if form.focused == form.fields.len() + STEP {
                    self.form = None;
                    return None;
                }
                if form.focused > form.fields.len() + STEP {
                    if let Some(action) = form.removal() {
                        self.confirmation = Some(Confirmation {
                            action,
                            label: form.removal_label(),
                            confirm: false,
                        });
                    }
                    return None;
                }
                if form
                    .fields
                    .get(form.focused)
                    .is_some_and(|field| matches!(field.value, Value::Text { .. }))
                {
                    form.move_focus(true);
                    return None;
                }
            }
            form.key(key);
            return None;
        }
        let items = self.items();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.selected = (self.selected + STEP).min(items.len().saturating_sub(STEP))
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                self.selected = self.selected.saturating_sub(STEP)
            }
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = items.len().saturating_sub(STEP),
            KeyCode::Enter => {
                if let Some(item) = items.get(self.selected).filter(|item| item.enabled) {
                    return self.activate(item.action.clone());
                }
            }
            KeyCode::Esc => {
                if self.page == Page::Root {
                    return self.activate(Action::Close);
                }
                self.page(Page::Root);
            }
            KeyCode::Char('q') => return self.activate(Action::Close),
            _ => {}
        }
        None
    }
    fn activate(&mut self, action: Action) -> Option<Effect> {
        let label = match action {
            Action::RemovePeer(_) => Some(REMOVE_PEER),
            Action::RemoveOrigin(_) => Some(REMOVE_ORIGIN),
            Action::RemoveFileRoot(_) => Some(REMOVE_FILE_ROOT),
            Action::Reload | Action::Close if self.dirty() || self.unresolved.is_some() => {
                Some(DISCARD)
            }
            _ => None,
        };
        if let Some(label) = label {
            self.confirmation = Some(Confirmation {
                action,
                label,
                confirm: false,
            });
            None
        } else {
            self.perform(action)
        }
    }
    fn perform(&mut self, action: Action) -> Option<Effect> {
        match action {
            Action::Page(page) => {
                self.page(page);
                if page == Page::Consumers {
                    return Some(Effect::Request(self.consumers_query()));
                }
            }
            Action::AddPeer => self.form = Some(Form::peer(None)),
            Action::EditPeer(hub) => {
                self.form = self
                    .draft
                    .as_ref()
                    .and_then(|draft| draft.peer(hub))
                    .map(|peer| Form::peer(Some(peer)))
            }
            Action::AddOrigin => self.form = Some(Form::origin()),
            Action::AddFileRoot => self.form = Some(Form::file_root(None)),
            Action::EditFileRoot(id) => {
                self.form = self
                    .draft
                    .as_ref()
                    .and_then(|draft| {
                        draft
                            .file_roots
                            .iter()
                            .find(|root| root.fields().root_id == id)
                    })
                    .map(|root| Form::file_root(Some(root)));
            }
            Action::AddExport(peer) => self.form = Some(Form::export(peer, self.entry, None)),
            Action::EditExport(peer, origin) => {
                self.form = self
                    .draft
                    .as_ref()
                    .and_then(|draft| draft.export(peer, origin))
                    .map(|policy| Form::export(peer, self.entry, Some(policy)))
            }
            Action::RemovePeer(_)
            | Action::RemoveOrigin(_)
            | Action::RemoveExport(_, _)
            | Action::RemoveFileRoot(_) => self.remove(action),
            Action::Apply => return self.proposal().map(Effect::Request),
            Action::RefreshConsumers => {
                return Some(Effect::Request(self.consumers_query()));
            }
            Action::Reload => return Some(Effect::Request(self.query(false))),
            Action::Check => return Some(Effect::Request(self.query(true))),
            Action::Close => return Some(Effect::Close),
            Action::None => {}
        }
        None
    }
}
