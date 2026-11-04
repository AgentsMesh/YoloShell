use super::*;
use crate::worker::{NativeControlDigest, NativeControlWitness};
use crate::{model::WriterCapability, protocol::RequestEnvelope};
use yolo_agent_control_contract::{NativeControlWitnessFields, binding_digest};
use yolo_runtime::agent::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};

#[path = "../test_support/native_control/constants.rs"]
mod constants;
#[path = "../test_support/native_control/fixtures.rs"]
mod fixtures;
#[path = "../test_support/native_control/server.rs"]
mod server;
use constants::*;
use fixtures::*;
use server::ReplyServer;

#[path = "native_control_interrupt_tests.rs"]
mod interrupt;
#[path = "native_control_observation_tests.rs"]
mod observation;
#[path = "native_control_receipt_tests.rs"]
mod receipt;
#[path = "native_control_submit_tests.rs"]
mod submit;
