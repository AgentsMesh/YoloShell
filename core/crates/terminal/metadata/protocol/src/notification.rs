use serde::{Deserialize, Serialize};

use crate::{constants::*, valid_text};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    Bell,
    Desktop,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Fields")]
pub struct TerminalNotification {
    pub title: String,
    pub body: String,
    pub kind: NotificationKind,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fields {
    title: String,
    body: String,
    kind: NotificationKind,
}

impl TryFrom<Fields> for TerminalNotification {
    type Error = &'static str;

    fn try_from(fields: Fields) -> Result<Self, Self::Error> {
        let notification = Self {
            title: fields.title,
            body: fields.body,
            kind: fields.kind,
        };
        notification.validate()?;
        Ok(notification)
    }
}

impl TerminalNotification {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !valid_text(&self.title, MAX_NOTIFICATION_TITLE_BYTES)
            || !valid_text(&self.body, MAX_NOTIFICATION_BODY_BYTES)
            || (self.kind == NotificationKind::Bell
                && (!self.title.is_empty() || !self.body.is_empty()))
        {
            return Err(INVALID_NOTIFICATION);
        }
        Ok(())
    }
}
