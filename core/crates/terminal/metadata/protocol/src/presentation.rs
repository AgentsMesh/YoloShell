use serde::{Deserialize, Deserializer, Serialize};

use crate::{TerminalNotification, constants::*, valid_text};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Fields")]
pub struct TerminalPresentation {
    pub title: Option<String>,
    pub notification_sequence: u64,
    pub notification: Option<TerminalNotification>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fields {
    #[serde(deserialize_with = "required_nullable")]
    title: Option<String>,
    notification_sequence: u64,
    #[serde(deserialize_with = "required_nullable")]
    notification: Option<TerminalNotification>,
}

fn required_nullable<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(deserializer)
}

impl TryFrom<Fields> for TerminalPresentation {
    type Error = &'static str;

    fn try_from(fields: Fields) -> Result<Self, Self::Error> {
        let presentation = Self {
            title: fields.title,
            notification_sequence: fields.notification_sequence,
            notification: fields.notification,
        };
        presentation.validate()?;
        Ok(presentation)
    }
}

impl TerminalPresentation {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self
            .title
            .as_ref()
            .is_some_and(|title| !valid_text(title, MAX_TITLE_BYTES))
            || (self.notification_sequence == u64::default()) != self.notification.is_none()
        {
            return Err(INVALID_PRESENTATION);
        }
        if let Some(notification) = &self.notification {
            notification.validate()?;
        }
        Ok(())
    }
}
