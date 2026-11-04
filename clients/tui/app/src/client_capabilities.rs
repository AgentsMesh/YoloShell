use crate::{State, constants::CLIENT_STARTUP_FAILURE_LABEL};
use yolo_tui_descriptor::tui_descriptor;
use yolo_builtin_client_contract::{CapabilityNegotiation, ClientCapability, negotiate as negotiate_contract};
use yolo_hub_rpc_protocol::Hello;

pub(crate) const CHAT_UNAVAILABLE: &str =
    "This Hub does not support Chat; local drafts are retained";
pub(crate) const AGENT_CONTROL_UNAVAILABLE: &str = "This Hub does not support agent control";
pub(crate) const HUB_MANAGEMENT_UNAVAILABLE: &str =
    "This Hub does not support configuration management";
pub(crate) const TERMINAL_CONTROL_UNAVAILABLE: &str =
    "Read Only · terminal input control is unavailable";
pub(crate) const TERMINAL_LIFECYCLE_UNAVAILABLE: &str =
    "This Hub does not allow creating or changing terminals";

pub(crate) fn require(state: &mut State, capability: ClientCapability, message: &str) -> bool {
    if state.client_capability_enabled(capability) {
        true
    } else {
        state.status = message.into();
        false
    }
}

/// Reject an incompatible live Hub before opening local view state or entering raw mode.
pub(crate) async fn open(options: &crate::Options) -> anyhow::Result<CapabilityNegotiation> {
    let deadline = tokio::time::Instant::now() + crate::constants::RPC_TIMEOUT;
    let client = options.endpoint.connect(options.hub, deadline).await?;
    let capabilities = negotiate(client.hello()).map_err(anyhow::Error::msg)?;
    anyhow::ensure!(capabilities.can_start(), startup_failure(&capabilities));
    Ok(capabilities)
}

/// The live Hello is projected at the client boundary, never into Hub state.
pub(crate) fn install(state: &mut State, hello: &Hello) -> bool {
    match negotiate(hello) {
        Ok(capabilities) => {
            if !capabilities.can_start() {
                state.client_startup_failure = Some(startup_failure(&capabilities));
            }
            state.client_capabilities = Some(capabilities);
        }
        Err(error) => state.client_startup_failure = Some(error),
    }
    if state.client_startup_failure.is_some() {
        state.status = CLIENT_STARTUP_FAILURE_LABEL.into();
        false
    } else {
        if !state.client_capability_enabled(ClientCapability::HubManagement) {
            state.hub_manager = None;
            if state.mode == crate::Mode::HubConfiguration {
                state.mode = crate::Mode::Inventory;
            }
        }
        true
    }
}

fn negotiate(hello: &Hello) -> Result<CapabilityNegotiation, String> {
    Ok(negotiate_contract(tui_descriptor().api(), hello))
}

fn startup_failure(capabilities: &CapabilityNegotiation) -> String {
    format!(
        "TUI startup requirements unavailable (protocol={}, missing={:?})",
        capabilities.protocol_matches,
        capabilities.missing_for_startup()
    )
}

impl State {
    pub(crate) fn client_capability_enabled(&self, capability: ClientCapability) -> bool {
        self.client_capabilities
            .as_ref()
            .is_some_and(|capabilities| capabilities.supports(capability))
    }
}
