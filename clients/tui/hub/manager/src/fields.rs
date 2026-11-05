use crate::constants::*;
use yolo_tui_line_input::Cursor;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FieldId {
    Hub,
    Key,
    Ssh,
    Socket,
    Import,
    Origin,
    Observe,
    ReadTerminal,
    WriteTerminal,
    Lifecycle,
    FileRead,
    AgentControl,
    ConversationRead,
    PresentationRead,
    PresentationWrite,
    Reexports,
    RootId,
    RootName,
    RootPath,
}
#[derive(Clone, Debug)]
pub(crate) enum Value {
    Text { text: String, cursor: Cursor },
    Toggle(bool),
    Stepper(u8),
}
#[derive(Clone, Debug)]
pub(crate) struct Field {
    pub id: FieldId,
    pub value: Value,
    pub read_only: bool,
}
impl Field {
    pub fn text(id: FieldId, value: impl Into<String>, read_only: bool) -> Self {
        let text = value.into();
        Self {
            id,
            value: Value::Text {
                text,
                cursor: Cursor::default(),
            },
            read_only,
        }
    }
    pub fn flag(id: FieldId, value: bool, read_only: bool) -> Self {
        Self {
            id,
            value: Value::Toggle(value),
            read_only,
        }
    }
    pub fn label(&self) -> &'static str {
        match self.id {
            FieldId::Hub => HUB_ID,
            FieldId::Key => PUBLIC_KEY,
            FieldId::Ssh => SSH_TARGET,
            FieldId::Socket => REMOTE_SOCKET,
            FieldId::Import => IMPORTS,
            FieldId::Origin => ORIGIN,
            FieldId::Observe => OBSERVE,
            FieldId::ReadTerminal => READ_TERMINAL,
            FieldId::WriteTerminal => WRITE_TERMINAL,
            FieldId::Lifecycle => LIFECYCLE,
            FieldId::FileRead => FILE_READ,
            FieldId::AgentControl => AGENT_CONTROL,
            FieldId::ConversationRead => CONVERSATION_READ,
            FieldId::PresentationRead => PRESENTATION_READ,
            FieldId::PresentationWrite => PRESENTATION_WRITE,
            FieldId::Reexports => REEXPORTS,
            FieldId::RootId => FILE_ROOT_ID,
            FieldId::RootName => FILE_ROOT_NAME,
            FieldId::RootPath => FILE_ROOT_PATH,
        }
    }
    pub fn display(&self) -> String {
        match &self.value {
            Value::Text { text, .. } => text.clone(),
            Value::Toggle(value) => if *value { CHECKED } else { UNCHECKED }.into(),
            Value::Stepper(value) => value.to_string(),
        }
    }
}
