use crate::{
    ControlContractError, NativeControlWitness, NativeInteraction, binding_digest,
    constants::{MAX_BINDING_TEXT_BYTES, NATIVE_CONTROL_SCHEMA_VERSION},
};
use serde::{Deserialize, Serialize};
use yolo_agent_contract::{AgentBinding, AgentState};

/// A provider turn-journal record that ends the turn. The Worker reads it from
/// the pinned transcript after the Stop hook result is final (`completed`) or
/// when a provider aborts a turn without a Stop hook (`aborted`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeTurnEnd {
    pub turn_id: String,
    pub kind: NativeTurnEndKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTurnEndKind {
    Completed,
    Aborted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ObservationFields")]
pub struct NativeControlObservation {
    schema_version: u16,
    binding: AgentBinding,
    witness: NativeControlWitness,
    prompt_input_ready: bool,
    turn_end: Option<NativeTurnEnd>,
    interaction: Option<NativeInteraction>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationFields {
    schema_version: u16,
    binding: AgentBinding,
    witness: NativeControlWitness,
    prompt_input_ready: bool,
    #[serde(default)]
    turn_end: Option<NativeTurnEnd>,
    #[serde(default)]
    interaction: Option<NativeInteraction>,
}
impl NativeControlObservation {
    pub fn new(
        binding: AgentBinding,
        witness: NativeControlWitness,
        prompt_input_ready: bool,
    ) -> Result<Self, ControlContractError> {
        Self::new_with_turn_end(binding, witness, prompt_input_ready, None)
    }

    /// Readiness may come from an idle PTY or from a verified provider
    /// turn-ending record. A record never authorizes an effect by itself; the
    /// Worker revalidates it at submission time.
    pub fn new_with_turn_end(
        binding: AgentBinding,
        witness: NativeControlWitness,
        prompt_input_ready: bool,
        turn_end: Option<NativeTurnEnd>,
    ) -> Result<Self, ControlContractError> {
        Self::new_with_interaction(binding, witness, prompt_input_ready, turn_end, None)
    }

    /// The pending interaction is an observation, not a response capability.
    /// The Worker revalidates its exact identity when a response is delivered.
    pub fn new_with_interaction(
        binding: AgentBinding,
        witness: NativeControlWitness,
        prompt_input_ready: bool,
        turn_end: Option<NativeTurnEnd>,
        interaction: Option<NativeInteraction>,
    ) -> Result<Self, ControlContractError> {
        let fields = witness.fields();
        let digest = binding_digest(&binding)?;
        let ready_without_idle = prompt_input_ready && binding.state != AgentState::WaitingForInput;
        let valid_turn_end = turn_end.as_ref().is_none_or(|end| {
            prompt_input_ready
                && matches!(
                    binding.state,
                    AgentState::WaitingForInput | AgentState::Active | AgentState::Running
                )
                && valid_turn_id(&end.turn_id)
        });
        if binding.terminal_id != fields.terminal_id
            || binding.kind != fields.kind
            || binding.process_pid != Some(fields.process_pid)
            || binding.process_start_time != Some(fields.process_start_time)
            || digest != fields.binding_digest
            || (ready_without_idle && turn_end.is_none())
            || !valid_turn_end
        {
            return Err(ControlContractError::BindingMismatch);
        }
        Ok(Self {
            schema_version: NATIVE_CONTROL_SCHEMA_VERSION,
            binding,
            witness,
            prompt_input_ready,
            turn_end,
            interaction,
        })
    }
    pub fn binding(&self) -> &AgentBinding {
        &self.binding
    }
    pub fn witness(&self) -> &NativeControlWitness {
        &self.witness
    }
    /// An observed input precondition, never a grant or a promise of delivery.
    pub fn prompt_input_ready(&self) -> bool {
        self.prompt_input_ready
    }
    pub fn turn_end(&self) -> Option<&NativeTurnEnd> {
        self.turn_end.as_ref()
    }
    pub fn interaction(&self) -> Option<&NativeInteraction> {
        self.interaction.as_ref()
    }
}
impl TryFrom<ObservationFields> for NativeControlObservation {
    type Error = ControlContractError;
    fn try_from(fields: ObservationFields) -> Result<Self, Self::Error> {
        if fields.schema_version != NATIVE_CONTROL_SCHEMA_VERSION {
            return Err(ControlContractError::UnsupportedSchema);
        }
        Self::new_with_interaction(
            fields.binding,
            fields.witness,
            fields.prompt_input_ready,
            fields.turn_end,
            fields.interaction,
        )
    }
}

fn valid_turn_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_BINDING_TEXT_BYTES
        && !value.chars().any(char::is_control)
}
