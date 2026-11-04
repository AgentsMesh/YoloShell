//! Exact startup-launch authority for agent publication. The Worker, Host
//! control and provider ingresses retain one fence across fenced writes.

use anyhow::{Context, Result, ensure};
use yolo_paths::namespace_lease::NamespaceLease;
use yolo_paths::{Paths, identity::FileIdentity};
use yolo_startup_checkpoint_contract::{
    StartupCheckpointEvidence, StartupLaunchFence, StartupStatus, from_evidence, matches_runtime,
};
use yolo_terminal_model::{RuntimeRecord, WorkerLaunchSpec};

const INVALID: &str = "agent publication lost its exact startup launch authority";
const LAUNCH_READ: &str = "read agent publication launch authority";
const CHECKPOINT_READ: &str = "read agent publication startup checkpoint";

pub struct AgentLaunchFence {
    launch: Option<(WorkerLaunchSpec, FileIdentity)>,
}

impl AgentLaunchFence {
    pub fn capture(
        namespace: &NamespaceLease,
        paths: &Paths,
        runtime: &RuntimeRecord,
    ) -> Result<Self> {
        let path = paths.worker_launch(runtime.terminal_id);
        let launch = if namespace.file_is_missing(&path)? {
            None
        } else {
            Some(
                namespace
                    .read_json_with_identity(&path)
                    .context(LAUNCH_READ)?,
            )
        };
        let fence = Self { launch };
        fence.verify(namespace, paths, runtime)?;
        Ok(fence)
    }

    pub fn verify(
        &self,
        namespace: &NamespaceLease,
        paths: &Paths,
        runtime: &RuntimeRecord,
    ) -> Result<()> {
        let path = paths.worker_launch(runtime.terminal_id);
        let Some((launch, identity)) = &self.launch else {
            ensure!(namespace.file_is_missing(&path)?, INVALID);
            return Ok(());
        };
        let current: (WorkerLaunchSpec, FileIdentity) = namespace
            .read_json_with_identity(&path)
            .context(LAUNCH_READ)?;
        ensure!(current.0 == *launch && current.1 == *identity, INVALID);
        let checkpoint: StartupCheckpointEvidence = namespace
            .read_json(&paths.worker_startup_checkpoint(runtime.terminal_id))
            .context(CHECKPOINT_READ)?;
        yolo_startup_checkpoint_contract::validation::validate(&checkpoint)?;
        // Resume retains its own launch until the new provider callback is authoritative.
        ensure!(
            matches!(
                checkpoint.status,
                StartupStatus::Active | StartupStatus::RuntimeActivated
            ) && checkpoint.successor.is_none()
                && matches_runtime(&checkpoint, runtime)?
                && from_evidence(&checkpoint) == StartupLaunchFence::from_launch(launch)?
                && launch.socket_path == runtime.socket_path
                && launch.auth_token == runtime.auth_token
                && launch.command == runtime.command
                && launch.args == runtime.args
                && launch.cwd == runtime.cwd,
            INVALID
        );
        Ok(())
    }
}
