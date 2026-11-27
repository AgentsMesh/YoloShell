use super::codex_hook::CodexHookIngressSupervisor;
use super::{Paths, WorkerLaunchSpec, WorkerNamespace, process};

pub(super) async fn start(
    namespace: &WorkerNamespace,
    paths: &Paths,
    launch: &WorkerLaunchSpec,
    hook_executable: &std::path::Path,
    shell: process::ProcessIdentity,
    agent_registry: std::sync::Arc<crate::agent::AgentRegistry>,
) -> anyhow::Result<CodexHookIngressSupervisor> {
    let supervisor = CodexHookIngressSupervisor::new();
    supervisor
        .start_native(
            namespace,
            paths,
            launch.terminal_id,
            launch.runtime_epoch,
            shell,
            hook_executable,
            agent_registry,
        )
        .await?;
    Ok(supervisor)
}
