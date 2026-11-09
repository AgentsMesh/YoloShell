pub(crate) const WORKSPACE_ID_FIELD: &str = yolo_hub_identity::field::WORKSPACE_ID;
pub(crate) const TERMINAL_ID_FIELD: &str = "terminal_id";
pub(crate) const RUN_ID_FIELD: &str = yolo_hub_identity::field::RUN_ID;
pub(crate) const HOST_ID_FIELD: &str = "host_id";
pub(crate) const PROFILE_ID_FIELD: &str = "profile_id";
pub(crate) const PATH_FIELD: &str = "path";
pub(crate) const ADDRESS_FIELD: &str = "address";
pub(crate) const REQUEST_ID_FIELD: &str = "request_id";
/// Structural envelopes that carry a command's own identity. User payloads
/// (`arguments`, `input`, `context`, `prompt`, ...) are intentionally absent.
pub(crate) const IDENTITY_ENVELOPES: &[&str] = &[
    "request",
    "command",
    "selection",
    "creation",
    "change",
    "address",
    "target",
    "parent",
    "selector",
    "reference",
    "execution",
    "terminal",
    "call",
    "profile",
];
pub(crate) const TRUST_SEPARATOR: char = '=';
pub(crate) const MAX_TRUSTED_AUDIT_SIGNERS: usize = 32;
pub(crate) const HEX_DIGITS_PER_BYTE: usize = 2;
pub(crate) const TRUST_INVALID: &str =
    "audit trust must be HUB_ID=64-character hex Ed25519 public keys without duplicates";
pub(crate) const AUDIT_SIGNING_FAILED: &str = "audit record signing failed";
pub(crate) const SIGNATURE_VALID: &str = "valid";
pub(crate) const SIGNATURE_INVALID: &str = "invalid";
pub(crate) const SIGNATURE_UNSIGNED: &str = "unsigned";
