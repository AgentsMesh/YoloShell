//! Private, coherent native control observations; these DTOs perform no effects.
//! Parsing proves shape and identity/digest coherence, never process custody or
//! permission. The trusted producer captures authority under its native guard;
//! the consumer must revalidate that source, exact boot, runtime and writer at use.

mod constants;
mod digest;
mod error;
mod interaction;
mod observation;
mod validation;
mod witness;

pub use constants::{
    MAX_BINDING_TEXT_BYTES, MAX_BOOT_ID_BYTES, MAX_INTERACTION_CHOICE_ID_BYTES,
    MAX_INTERACTION_CHOICE_LABEL_BYTES, MAX_INTERACTION_CHOICES, MAX_INTERACTION_PROMPT_BYTES,
    MAX_PROCESS_PID, NATIVE_CONTROL_SCHEMA_VERSION, PROMPT_INPUT_READY_FIELD,
};
pub use digest::{authority_digest, binding_digest};
pub use error::ControlContractError;
pub use interaction::{NativeInteraction, NativeInteractionChoice, NativeInteractionKind};
pub use observation::{NativeControlObservation, NativeTurnEnd, NativeTurnEndKind};
pub use witness::{NativeControlWitness, NativeControlWitnessFields};
