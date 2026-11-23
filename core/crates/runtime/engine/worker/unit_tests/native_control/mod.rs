use super::*;
use crate::worker::native_control::{
    NativeControlLookupRequest, NativeControlOutcome, NativeInterruptRequest,
    NativeObservationRequest,
};
use yolo_agent_control_contract::{NativeControlWitness, binding_digest};

#[path = "../../test_support/unit/claude_hook/control_source_fixture.rs"]
mod source;
use source::NativeSource;
#[path = "../../test_support/unit/qwen_hook/control_source_fixture.rs"]
mod generic_source;
#[path = "../../test_support/unit/native_control/constants.rs"]
mod constants;
#[path = "../../test_support/unit/native_control/initialize.rs"]
mod initialize;
#[path = "../../test_support/unit/native_control/requests.rs"]
mod requests;
use self::constants::*;
use requests::*;

mod admission;
mod authorization;
mod corruption;
mod execution;
mod generic_hook_control;
mod observation;
mod receipt;
mod respond;
mod submit_authorization;
mod submit_execution;
mod submit_fencing;
#[path = "../../test_support/unit/native_control/submit.rs"]
mod submit_support;
