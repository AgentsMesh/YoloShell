use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::AgentTarget;

use crate::observed_provider::{valid_text, valid_token};
use crate::{
    MAX_CHOICE_ID_BYTES, MAX_CHOICE_LABEL_BYTES, MAX_INTERACTION_CHOICES,
    MAX_INTERACTION_PROMPT_BYTES, ProtocolError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionKind {
    Approval,
    Choice,
    Text,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteractionChoice {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteractionReference {
    pub target: AgentTarget,
    pub id: Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "InteractionFields", into = "InteractionFields")]
pub struct Interaction(InteractionFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InteractionFields {
    reference: InteractionReference,
    kind: InteractionKind,
    choices: Vec<InteractionChoice>,
    /// Bounded display text for the question or tool request. Older payloads
    /// that predate interaction prompts omit it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    prompt: String,
}

impl Interaction {
    pub fn new(
        reference: InteractionReference,
        kind: InteractionKind,
        choices: Vec<InteractionChoice>,
    ) -> Result<Self, ProtocolError> {
        Self::with_prompt(reference, kind, choices, String::new())
    }

    pub fn with_prompt(
        reference: InteractionReference,
        kind: InteractionKind,
        choices: Vec<InteractionChoice>,
        prompt: String,
    ) -> Result<Self, ProtocolError> {
        InteractionFields {
            reference,
            kind,
            choices,
            prompt,
        }
        .try_into()
    }

    pub fn reference(&self) -> InteractionReference {
        self.0.reference
    }

    pub fn kind(&self) -> InteractionKind {
        self.0.kind
    }

    pub fn choices(&self) -> &[InteractionChoice] {
        &self.0.choices
    }

    pub fn prompt(&self) -> &str {
        &self.0.prompt
    }
}

impl TryFrom<InteractionFields> for Interaction {
    type Error = ProtocolError;

    fn try_from(fields: InteractionFields) -> Result<Self, Self::Error> {
        if fields.reference.id.is_nil()
            || fields.choices.len() > MAX_INTERACTION_CHOICES
            || fields.prompt.len() > MAX_INTERACTION_PROMPT_BYTES
            || fields.prompt.contains('\0')
        {
            return Err(ProtocolError::InvalidInteraction);
        }
        let expects_choices = fields.kind != InteractionKind::Text;
        if expects_choices == fields.choices.is_empty() {
            return Err(ProtocolError::InvalidInteraction);
        }
        let mut ids = BTreeSet::new();
        for choice in &fields.choices {
            if !valid_token(&choice.id, MAX_CHOICE_ID_BYTES)
                || !valid_text(&choice.label, MAX_CHOICE_LABEL_BYTES)
                || !ids.insert(&choice.id)
            {
                return Err(ProtocolError::InvalidInteraction);
            }
        }
        Ok(Self(fields))
    }
}

impl From<Interaction> for InteractionFields {
    fn from(interaction: Interaction) -> Self {
        interaction.0
    }
}
