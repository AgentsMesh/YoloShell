use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::AgentTarget;

use crate::observed_provider::valid_token;
use crate::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentSnapshot, Freshness, InteractionKind, InteractionReference,
    MAX_CHOICE_ID_BYTES, MAX_INTERACTION_TEXT_BYTES, ObservationCursor, ProtocolError,
    Reachability,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InteractionResponse {
    Choice { choice: String },
    Text { text: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentAction {
    Interrupt {},
    SubmitPrompt {
        prompt: yolo_agent_input::Prompt,
    },
    Respond {
        interaction: InteractionReference,
        response: InteractionResponse,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AgentControlFields", into = "AgentControlFields")]
pub struct AgentControlRequest(AgentControlFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentControlFields {
    pub schema_version: u16,
    pub request_id: Uuid,
    pub target: AgentTarget,
    pub expected_cursor: ObservationCursor,
    pub action: AgentAction,
}

impl AgentControlRequest {
    pub fn new(fields: AgentControlFields) -> Result<Self, ProtocolError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &AgentControlFields {
        &self.0
    }

    /// Checks observation preconditions only, not caller authorization or write ownership.
    /// The origin must perform those checks and execute atomically with this validation.
    pub fn validate_against(&self, snapshot: &AgentSnapshot) -> Result<(), ProtocolError> {
        let current = snapshot.fields();
        if self.0.target != current.cursor.target() {
            return Err(ProtocolError::TargetMismatch);
        }
        if self.0.expected_cursor != current.cursor {
            return Err(ProtocolError::RevisionMismatch);
        }
        if current.freshness != Freshness::Current {
            return Err(ProtocolError::StaleObservation);
        }
        if current.reachability != Reachability::Reachable {
            return Err(ProtocolError::Unreachable);
        }
        match &self.0.action {
            AgentAction::Interrupt {} => current.capabilities.interrupt.require_available(),
            AgentAction::SubmitPrompt { .. } => {
                current.capabilities.submit_prompt.require_available()
            }
            AgentAction::Respond {
                interaction,
                response,
            } => {
                current
                    .capabilities
                    .respond_interaction
                    .require_available()?;
                let pending = current
                    .interaction
                    .as_ref()
                    .ok_or(ProtocolError::UnknownInteraction)?;
                if pending.reference() != *interaction {
                    return Err(ProtocolError::UnknownInteraction);
                }
                let allowed = match response {
                    InteractionResponse::Choice { choice } => {
                        pending.choices().iter().any(|c| c.id == *choice)
                    }
                    InteractionResponse::Text { .. } => pending.kind() == InteractionKind::Text,
                };
                if allowed {
                    Ok(())
                } else {
                    Err(ProtocolError::InvalidResponse)
                }
            }
        }
    }
}

impl TryFrom<AgentControlFields> for AgentControlRequest {
    type Error = ProtocolError;

    fn try_from(fields: AgentControlFields) -> Result<Self, Self::Error> {
        if fields.schema_version != AGENT_PROTOCOL_SCHEMA_VERSION {
            return Err(ProtocolError::UnsupportedSchema);
        }
        if fields.request_id.is_nil() {
            return Err(ProtocolError::InvalidRequest);
        }
        if fields.target != fields.expected_cursor.target() {
            return Err(ProtocolError::TargetMismatch);
        }
        if let AgentAction::Respond {
            interaction,
            response,
        } = &fields.action
        {
            if interaction.target != fields.target || interaction.id.is_nil() {
                return Err(ProtocolError::InvalidInteraction);
            }
            let valid = match response {
                InteractionResponse::Choice { choice } => valid_token(choice, MAX_CHOICE_ID_BYTES),
                InteractionResponse::Text { text } => {
                    text.len() <= MAX_INTERACTION_TEXT_BYTES && !text.contains('\0')
                }
            };
            if !valid {
                return Err(ProtocolError::InvalidResponse);
            }
        }
        Ok(Self(fields))
    }
}

impl From<AgentControlRequest> for AgentControlFields {
    fn from(request: AgentControlRequest) -> Self {
        request.0
    }
}
