pub(super) const ROUTER_SCOPE_ERROR: &str = "Codex router scope does not match Begin authority";
pub(super) const ROUTER_RUNTIME_ERROR: &str =
    "Codex router runtime does not match durable Begin authority";
pub(super) const ROUTER_ADMISSION_ERROR: &str = "Codex router durable admission is unavailable";
pub(super) const ROUTER_CONFLICT_ERROR: &str = "Codex router invocation authority conflicts";
pub(super) const ROUTER_LOCK_ERROR: &str = "Codex router authority lock is poisoned";
pub(super) const ROUTER_CAPACITY_ERROR: &str = "Codex router cannot reserve invocation authority";
pub(super) const ROUTER_ENTRY_RESERVATION: usize = 1;
pub(in crate::worker::codex_hook) const MAX_ACTIVE_CODEX_INVOCATIONS: usize = 64;
pub(super) const ROUTE_PUBLICATION_CAPACITY: usize = MAX_ACTIVE_CODEX_INVOCATIONS;
pub(super) const ROUTE_PUBLICATION_REVISION_INCREMENT: u64 = 1;
pub(super) const ROUTER_LEASE_INCREMENT: usize = 1;
