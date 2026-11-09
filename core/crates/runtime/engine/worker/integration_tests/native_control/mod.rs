use super::*;
use crate::worker::native_control::{
    NativeControlLookupRequest, NativeControlOutcome, NativeInterruptRequest,
    NativeObservationRequest,
};
use yolo_agent_control_contract::binding_digest;

#[path = "../../test_support/unit/claude_hook/control_source_fixture.rs"]
mod source;
use source::NativeSource;
#[path = "../../test_support/unit/native_control/constants.rs"]
mod constants;
#[path = "../../test_support/unit/native_control/initialize.rs"]
mod initialize;
#[path = "../../test_support/unit/native_control/requests.rs"]
mod requests;
#[path = "../../test_support/unit/native_control/submit.rs"]
mod submit_support;
use self::constants::*;
use requests::*;

mod submit_pty;
