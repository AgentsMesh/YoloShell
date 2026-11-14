use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_crypto::Sha256Digest;
use yolo_paths::identity::DurableFileVersion;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupStage {
    LaunchContract,
    StartupContainment,
    WorkerSocket,
    AgentRuntime,
    TerminalJournal,
    Pty,
    ChildProcess,
    BootstrapGate,
    RuntimeRecord,
    SecondaryCustody,
    CommandExec,
    BootstrapInput,
    RuntimeActivation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupPhase {
    Entered,
    Completed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupStatus {
    Active,
    RuntimeActivated,
    HostSettled,
    HostRetired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartupProcessIdentity {
    pub pid: u32,
    pub start_time: u64,
}

impl StartupProcessIdentity {
    pub const fn new(pid: u32, start_time: u64) -> Self {
        Self { pid, start_time }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartupLaunchFence {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub worker_id: Uuid,
    pub controller_generation: u64,
    pub launch_sha256: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartupPtyCustodyRollover {
    pub source_version: DurableFileVersion,
    #[serde(with = "yolo_crypto::sha256_digest::compact")]
    pub source_sha256: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartupCheckpointEvidence {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub worker_id: Uuid,
    pub controller_generation: u64,
    pub launch_sha256: Sha256Digest,
    pub boot_id: String,
    pub worker: StartupProcessIdentity,
    pub child: Option<StartupProcessIdentity>,
    pub stage: StartupStage,
    pub phase: StartupPhase,
    pub status: StartupStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub successor: Option<StartupLaunchFence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor_runtime_sha256: Option<Sha256Digest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_pty_custody_rollover: Option<StartupPtyCustodyRollover>,
}
