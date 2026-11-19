use super::{Paths, WorkerNamespace, WorkerState, lock_recover, process, supervision};
use anyhow::Result;
use portable_pty::CommandBuilder;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinSet;
use yolo_hook_ingress::{
    HookExecutable, HookIngressHost, HookIngressPump, HookIngressServer, MAX_HOOK_CLIENTS,
    OUTBOX_DIRECTORY, OUTBOX_ENV, OUTBOX_FILE, SOCKET_ENV,
};

/// Bootstrap environment for provider hooks that use the neutral ingress.
pub(crate) struct ProviderHookEnvironment {
    socket: PathBuf,
    outbox: PathBuf,
}

impl ProviderHookEnvironment {
    pub(crate) fn configure(&self, command: &mut CommandBuilder) {
        command.env(SOCKET_ENV, &self.socket);
        command.env(OUTBOX_ENV, &self.outbox);
    }
}

pub(super) struct ProviderHookServer {
    server: HookIngressServer,
    environment: ProviderHookEnvironment,
    executable: Arc<HookExecutable>,
}

impl ProviderHookServer {
    pub(crate) fn environment(&self) -> &ProviderHookEnvironment {
        &self.environment
    }
}

pub(super) async fn bind(
    namespace: &WorkerNamespace,
    paths: &Paths,
    terminal_id: uuid::Uuid,
    executable: PathBuf,
) -> Result<ProviderHookServer> {
    let socket = namespace.provider_hook_socket(terminal_id)?;
    let directory = paths
        .session_dir(terminal_id)
        .join(OUTBOX_DIRECTORY);
    namespace
        .lease()
        .ensure_directory_with_commit_fence(&directory, || namespace.verify())?;
    let outbox = directory.join(OUTBOX_FILE);
    yolo_hook_ingress::prune_dead(&outbox)?;
    let server = HookIngressServer::bind(
        namespace.lease_arc(),
        socket.clone(),
        MAX_HOOK_CLIENTS,
    )
    .await?;
    Ok(ProviderHookServer {
        server,
        environment: ProviderHookEnvironment { socket, outbox },
        executable: Arc::new(HookExecutable::open(executable)?),
    })
}

pub(super) async fn start(
    tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    server: ProviderHookServer,
    state: Arc<WorkerState>,
    shell: process::ProcessIdentity,
    wait_ready: bool,
) -> Result<HookIngressPump> {
    let (runtime_epoch, worker, boot_id, binding_path) = {
        let record = lock_recover(&state.record);
        (
            record.runtime_epoch,
            crate::agent::authority::AuthorityProcessIdentity {
                pid: record.worker_pid,
                start_time: record.worker_start_time,
            },
            record.boot_id.clone(),
            state.paths.agent_binding(record.terminal_id),
        )
    };
    let publisher = Arc::clone(&state);
    let current = Arc::clone(&state);
    let expected = worker;
    let host = HookIngressHost {
        terminal_id: state.terminal_id(),
        runtime_epoch,
        worker,
        boot_id,
        binding_path,
        namespace: state.namespace.lease_arc(),
        generation_gate: Arc::clone(&state.agent_generation_gate),
        provider_runtime: Arc::clone(&state.provider_runtime),
        process_source: Arc::new(crate::agent::RuntimeProcessSource::new(
            state.agent_registry.process_environment_keys(),
        )),
        shell,
        codecs: Arc::new(state.provider_runtime.hook_codecs()),
        publish: Arc::new(move |event_type, binding| {
            publisher.publish_agent_event(event_type, binding);
        }),
        runtime_current: Arc::new(move || {
            lock_recover(&current.record).matches_worker_runtime(
                runtime_epoch,
                expected.pid,
                expected.start_time,
            )
        }),
        outbox: server.environment.outbox.clone(),
    };
    let (mut pump, future) = server.server.into_pump(host, server.executable);
    supervision::spawn_task(tasks, yolo_hook_ingress::HOOK_ADMISSION_TASK, future);
    if wait_ready {
        pump.wait_ready().await?;
    } else {
        let readiness = pump.take_readiness()?;
        supervision::spawn_task(
            tasks,
            yolo_hook_ingress::HOOK_READINESS_RETENTION_TASK,
            async move {
                let _ = readiness.await;
                Ok(())
            },
        );
    }
    Ok(pump)
}
