pub const HUB_SOCKET_FILE: &str = yolo_paths::constants::HUB_SOCKET_FILE_NAME;
pub const HUB_IDENTITY_FILE: &str = "identity.json";
pub const PEER_SOCKET_FILE: &str = "peer.sock";
pub const STATE_CONTRACT_FILE: &str = "runtime-contract.json";
#[cfg(test)]
pub(crate) const HUB_PROTOCOL_FIELD: &str = "hub_protocol";
pub(crate) const HUB_LOCK_FILE: &str = "hub.lock";
pub(crate) const STATE_SCHEMA_VERSION: u16 = 2;
pub(crate) const MAX_LOCK_BYTES: u64 = 4_096;
pub(crate) const STATE_NAMESPACE: &str = "Hub state";
pub(crate) const ALREADY_RUNNING: &str = "Hub state is already in use";
pub(crate) const CLONED_STATE: &str =
    "Hub state was moved or cloned; explicit reenrollment is required";
pub(crate) const INVALID_SCHEMA: &str = "unsupported Hub identity schema";
pub(crate) const KEY_FAILURE: &str = "Hub identity key is invalid";
pub(crate) const WRONG_BACKEND: &str = "Hub identity is bound to another local Host";
pub(crate) const HOST_COORDINATOR: &str = "federated-hub";
