use yolo_hub_agent_protocol::{AgentControlRequest, AgentSnapshot, ProtocolError};
use yolo_hub_identity::{HubId, RuntimeTarget, TerminalAddress};
#[path = "native_rpc_diagnostics.rs"]
mod rpc_classification;
pub(crate) use rpc_classification::{RpcResult, RpcStage};

const OBSERVATION_MESSAGE: &str = "Native control admission observation";
const REJECTION_MESSAGE: &str = "Native control rejected before Worker dispatch";
const RPC_RESULT_MESSAGE: &str = "Native control Worker RPC result";
const OBSERVATION_UNAVAILABLE_MESSAGE: &str =
    "Native control observation unavailable during catalogue refresh";

/// This is the classification of an existing read, including background reads
/// that can change a public cursor before the next control request arrives.
pub(crate) fn catalogue_unavailable(address: TerminalAddress, stage: ObservationStage) {
    tracing::debug!(message = OBSERVATION_UNAVAILABLE_MESSAGE, ?address, ?stage,);
}

/// Classify only an already completed RPC. No underlying error, grant, digest,
/// capability, path, binding or source witness enters this record.
pub(crate) fn rpc_result(
    origin: HubId,
    request: &AgentControlRequest,
    stage: RpcStage,
    result: RpcResult,
) {
    tracing::debug!(
        message = RPC_RESULT_MESSAGE,
        hub = %origin,
        request_id = %request.fields().request_id,
        ?stage,
        ?result,
    );
}

pub(crate) use yolo_hub_agent_observe::ObservationStage;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Rejection {
    ObservationOnly,
    TerminalMismatch,
    Request(ProtocolError),
    WitnessUnavailable,
}

#[derive(Clone, Copy)]
pub(crate) struct ObservationContext<'a> {
    pub request: &'a AgentControlRequest,
    pub terminal: RuntimeTarget,
}

/// Only the requested session's already-required admission read is recorded.
/// Background refreshes never use this context or produce these diagnostics.
pub(crate) fn observation(origin: HubId, context: ObservationContext<'_>, stage: ObservationStage) {
    tracing::debug!(
        message = OBSERVATION_MESSAGE,
        hub = %origin,
        request_id = %context.request.fields().request_id,
        terminal = ?context.terminal,
        ?stage,
    );
}

/// All values are public identities or typed comparisons. Do not log grants,
/// private witnesses, paths, native session references, or underlying RPC errors.
pub(crate) fn rejected(
    origin: HubId,
    request: &AgentControlRequest,
    snapshot: &AgentSnapshot,
    terminal: RuntimeTarget,
    reason: Rejection,
) {
    tracing::warn!(
        message = REJECTION_MESSAGE,
        hub = %origin,
        request_id = %request.fields().request_id,
        expected_cursor = ?request.fields().expected_cursor,
        current_cursor = ?snapshot.cursor(),
        terminal_matches = (terminal == snapshot.fields().terminal),
        ?reason,
    );
}
