//! A provider interaction that is waiting for a typed response. The Worker
//! owns the pending request; the Hub only mirrors it in an observation.

use crate::constants::*;
use crate::ControlContractError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeInteractionKind {
    Approval,
    Choice,
    Text,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeInteractionChoice {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "InteractionFields", into = "InteractionFields")]
pub struct NativeInteraction(InteractionFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InteractionFields {
    #[serde(with = "yolo_crypto::serde_uuid")]
    id: Uuid,
    kind: NativeInteractionKind,
    choices: Vec<NativeInteractionChoice>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    prompt: String,
}

impl NativeInteraction {
    pub fn new(
        id: Uuid,
        kind: NativeInteractionKind,
        choices: Vec<NativeInteractionChoice>,
        prompt: String,
    ) -> Result<Self, ControlContractError> {
        InteractionFields {
            id,
            kind,
            choices,
            prompt,
        }
        .try_into()
    }

    pub fn id(&self) -> Uuid {
        self.0.id
    }

    pub fn kind(&self) -> NativeInteractionKind {
        self.0.kind
    }

    pub fn choices(&self) -> &[NativeInteractionChoice] {
        &self.0.choices
    }

    pub fn prompt(&self) -> &str {
        &self.0.prompt
    }
}

impl TryFrom<InteractionFields> for NativeInteraction {
    type Error = ControlContractError;

    fn try_from(fields: InteractionFields) -> Result<Self, Self::Error> {
        let expects_choices = fields.kind != NativeInteractionKind::Text;
        let mut ids = BTreeSet::new();
        if fields.id.is_nil()
            || fields.choices.len() > MAX_INTERACTION_CHOICES
            || expects_choices == fields.choices.is_empty()
            || fields.prompt.len() > MAX_INTERACTION_PROMPT_BYTES
            || fields.prompt.contains('\0')
            || fields.choices.iter().any(|choice| {
                !text(&choice.id, MAX_INTERACTION_CHOICE_ID_BYTES)
                    || !text(&choice.label, MAX_INTERACTION_CHOICE_LABEL_BYTES)
                    || !ids.insert(&choice.id)
            })
        {
            return Err(ControlContractError::InvalidInteraction);
        }
        Ok(Self(fields))
    }
}

impl From<NativeInteraction> for InteractionFields {
    fn from(interaction: NativeInteraction) -> Self {
        interaction.0
    }
}

fn text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}
