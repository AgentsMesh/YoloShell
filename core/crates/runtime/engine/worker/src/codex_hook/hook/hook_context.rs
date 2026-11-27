use super::authority_registry::CodexAuthorityRegistry;
use super::hook_policy::CodexHookPolicy;
use super::router::{CodexRouterError, CodexSessionRouter};
use crate::agent::ProcessSource;
use std::sync::Arc;

#[path = "../native/native_transaction.rs"]
mod native;
pub(super) use native::{NativeCodexHandoff, NativePreflightFailure};

#[derive(Clone)]
pub(in crate::worker::codex_hook) struct CodexHookContext {
    paths: crate::paths::Paths,
    registry: Arc<CodexAuthorityRegistry>,
    router: Arc<CodexSessionRouter>,
    policy: CodexHookPolicy,
    shell: crate::process::ProcessIdentity,
    process_source: Arc<dyn ProcessSource>,
    settlements: super::blocking_custody::BlockingSettlementCustody,
    generation: super::cancellation::IngressCancellation,
    native_namespace: Arc<crate::namespace_lease::NamespaceLease>,
    native_callbacks: Arc<tokio::sync::Mutex<()>>,
}

impl CodexHookContext {
    pub(super) fn new(
        paths: crate::paths::Paths,
        registry: Arc<CodexAuthorityRegistry>,
        router: Arc<CodexSessionRouter>,
        policy: CodexHookPolicy,
        shell: crate::process::ProcessIdentity,
        process_source: Arc<dyn ProcessSource>,
        settlements: super::blocking_custody::BlockingSettlementCustody,
        generation: super::cancellation::IngressCancellation,
        native_namespace: Arc<crate::namespace_lease::NamespaceLease>,
    ) -> Result<Self, CodexRouterError> {
        if router.terminal_id() != policy.terminal_id()
            || router.runtime_epoch() != policy.runtime_epoch()
        {
            return Err(CodexRouterError::Scope);
        }
        Ok(Self {
            paths,
            registry,
            router,
            policy,
            shell,
            process_source,
            settlements,
            generation,
            native_namespace,
            native_callbacks: Arc::new(tokio::sync::Mutex::new(())),
        })
    }
}
