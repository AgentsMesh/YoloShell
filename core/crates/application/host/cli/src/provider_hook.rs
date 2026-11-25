//! Unified provider hook entrypoint. The provider table decides which client
//! runs: the generic ingress, Codex's occurrence witness or Claude's bridge.
use anyhow::{Context, Result};
use std::path::PathBuf;
use yolo_runtime::agent::HookClient;

const UNKNOWN_PROVIDER: &str = "provider hook caller is not registered";

pub(super) fn client(provider: &str) -> Option<HookClient> {
    yolo_runtime::agent::builtin_provider_runtime().hook_client(provider)
}

pub(super) async fn run(provider: &str, home: PathBuf) -> Result<()> {
    let registry = yolo_runtime::agent::builtin_provider_runtime();
    match registry.hook_client(provider).context(UNKNOWN_PROVIDER)? {
        HookClient::CodexNative => {
            yolo_runtime::codex_hooks::run_native(yolo_runtime::paths::Paths::new(home))
        }
        HookClient::ClaudeNative => yolo_runtime::claude_hook_bridge::run().await,
        HookClient::Ingress => {
            let codec = registry.hook_codec(provider).context(UNKNOWN_PROVIDER)?;
            yolo_runtime::hook_ingress::run_hook(codec.as_ref()).await
        }
    }
}
