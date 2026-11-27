//! Provider-neutral hook ingress. A provider supplies a [`HookCodec`] and an
//! install profile; this crate owns the durable outbox, authenticated delivery,
//! ancestry admission, evidence fencing, generation application and binding
//! publication. Qwen Code retains its earlier provider-specific ingress until
//! it migrates to this engine.
mod admission;
mod client;
mod constants;
mod contract;
mod executable;
mod handler;
mod host;
mod outbox;
mod parent;
mod peer;
mod profile;
mod pump;
mod server;
mod transaction;

pub use client::ClientContext;
pub use constants::{
    FIRST_SEQUENCE, HOOK_ADMISSION_TASK, HOOK_READINESS_RETENTION_TASK, MAX_HOOK_CLIENTS,
    OUTBOX_DIRECTORY, OUTBOX_ENV, OUTBOX_FILE, SOCKET_ENV,
};
pub use contract::{
    HookProcessIdentity, HookRequest, HookResponse, HookScope, ACCEPTED, NOT_READY, REJECTED,
};
pub use executable::HookExecutable;
pub use host::HookIngressHost;
pub use outbox::prune_dead;
pub use profile::{CanonicalHookEvent, HookAncestry, HookCodec, HookIngressProfile};
pub use pump::HookIngressPump;
pub use server::HookIngressServer;

/// Entrypoint for a provider hook command (`yolo internal host provider-hook`).
/// The codec validates the payload before it enters the durable outbox, so a
/// permanently invalid payload cannot block later lifecycle events.
pub async fn run_hook(codec: &dyn HookCodec) -> anyhow::Result<()> {
    client::run(codec).await
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
