pub(super) const REGISTRATION_FILE: &str = ".socket-namespace.json";
pub(super) const INITIALIZATION_LOCK: &str = ".socket-namespace-init.lock";
pub(super) const REGISTRATION_SCHEMA: u16 = 2;
// Includes the current bounded filesystem UUID/export-handle identity.
pub(super) const REGISTRATION_MAX_BYTES: u64 = 8_192;
pub(super) const READ_SENTINEL: u64 = 1;
pub(super) const NAMESPACE: &str = "socket namespace registration";
pub(super) const INVALID_REGISTRATION: &str = "socket namespace registration is invalid";
pub(super) const REGISTRATION_BUSY: &str = "socket namespace registration is in use";
pub(super) const INITIALIZATION_BUSY: &str =
    "socket namespace registration initialization is in use";
pub(super) const REGISTRATION_MISSING: &str = "socket namespace registration is missing";
pub(super) const PROCESS_CHANGED: &str = "socket namespace registration belongs to another process";
