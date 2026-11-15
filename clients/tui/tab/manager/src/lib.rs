mod constants;
mod input;
mod render;

pub use render::render;
use uuid::Uuid;
use yolo_tui_tab::TabCatalog;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Select(Uuid),
    Create(String),
    Rename(Uuid, String),
    Remove(Uuid),
    Close,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Form {
    Create,
    Rename(Uuid),
    Remove(Uuid),
}

pub struct Manager {
    pub(crate) selected: Uuid,
    pub(crate) form: Option<Form>,
    pub(crate) text: String,
    pub(crate) cursor: yolo_tui_line_input::Cursor,
    pub(crate) message: String,
}

impl Manager {
    pub fn new(tabs: &TabCatalog) -> Self {
        Self {
            selected: tabs.active().id,
            form: None,
            text: String::new(),
            cursor: Default::default(),
            message: String::new(),
        }
    }
    pub fn create(&mut self) {
        self.form = Some(Form::Create);
        self.text.clear();
        self.cursor.reset();
    }
    pub fn rename(&mut self, tabs: &TabCatalog) {
        if let Some(entry) = tabs
            .entries()
            .iter()
            .find(|entry| entry.id == self.selected)
        {
            self.form = Some(Form::Rename(entry.id));
            self.text = entry.name.clone();
            self.cursor.select_all();
        }
    }
    pub fn error(&mut self, message: String) {
        self.message = message;
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
