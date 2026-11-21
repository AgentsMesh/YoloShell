use super::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_remote_broker::{BROKER_SUPERVISION_RECORD_VERSION, BrokerSupervisionStore};
use yolo_runtime::paths::{identity::FileIdentity, namespace_lease::NamespaceLease};
use yolo_runtime::process::{BootIdentityRelation, boot_id, boot_identity_relation};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub pid: u32,
    pub start_time: u64,
}
impl Identity {
    pub fn process(self) -> crate::ProcessIdentity {
        crate::ProcessIdentity {
            pid: self.pid,
            start_time: self.start_time,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Record {
    version: u16,
    pub launch_id: String,
    generation: String,
    boot_id: String,
    pub supervisor: Identity,
    launcher: Identity,
    pub broker: Option<Identity>,
    instance_id: Option<uuid::Uuid>,
    broker_host_id: Option<String>,
    phase: String,
}
pub struct ObservedRecord {
    namespace: NamespaceLease,
    path: PathBuf,
    file: FileIdentity,
    pub record: Record,
}
impl ObservedRecord {
    pub fn capture(
        paths: &BrokerPaths,
        registration: &SshBrokerProfile,
        status: &BrokerStatus,
        broker_pid: u32,
    ) -> Result<Self> {
        paths.verify_state()?;
        let store = BrokerSupervisionStore::open_expected(paths.state_dir(), paths.state_anchor())?;
        let mut names = Vec::new();
        store.visit_names_with_prefix(RECORD_PREFIX, |name| {
            if name
                .strip_prefix(RECORD_PREFIX)
                .and_then(|value| value.strip_suffix(RECORD_SUFFIX))
                .is_some_and(|id| id.parse::<uuid::Uuid>().is_ok())
            {
                names.push(name);
            }
        })?;
        ensure!(names.len() == ONE_SUPERVISOR, INVALID_CUSTODY);
        let directory = yolo_runtime::paths::directory::PrivateDirectory::open(paths.state_dir())?;
        let namespace = NamespaceLease::from_directories(vec![(RECORD_NAMESPACE, directory)])?;
        let path = paths.state_dir().join(&names[0]);
        let (record, file): (Record, FileIdentity) = namespace.read_json_with_identity(&path)?;
        ensure!(
            record.version == BROKER_SUPERVISION_RECORD_VERSION
                && record.generation == registration.config_revision
                && record.generation == status.config_revision
                && record.phase == RUNNING
                && status.adopted
                && record.instance_id == Some(status.instance_id)
                && record.broker_host_id.as_deref() == Some(registration.profile.profile_id())
                && record.broker_host_id.as_deref() == Some(status.host_id.as_str())
                && boot_identity_relation(&record.boot_id, &boot_id())
                    == BootIdentityRelation::Same,
            INVALID_CUSTODY
        );
        let authority: yolo_remote_broker::BrokerLaunchAuthority = record.launch_id.parse()?;
        ensure!(
            authority.to_string() == record.launch_id
                && names[0] == format!("{RECORD_PREFIX}{}{RECORD_SUFFIX}", record.launch_id),
            INVALID_CUSTODY
        );
        authority.verify(paths)?;
        let broker = record
            .broker
            .ok_or_else(|| anyhow::anyhow!(INVALID_CUSTODY))?;
        let provenance = yolo_runtime::agent::process_environment_value(
            broker.pid,
            yolo_runtime::cli::constants::BROKER_SUPERVISION_AUTHORITY_ENVIRONMENT,
        )?;
        ensure!(
            provenance.as_deref() == Some(record.launch_id.as_str()),
            INVALID_CUSTODY
        );
        ensure!(
            broker.pid == broker_pid
                && broker.process().matches()
                && record.supervisor.process().matches()
                && broker != record.supervisor
                && broker != record.launcher,
            INVALID_CUSTODY
        );
        let observed = Self {
            namespace,
            path,
            file,
            record,
        };
        observed.verify()?;
        Ok(observed)
    }
    pub fn verify(&self) -> Result<()> {
        self.namespace.verify()?;
        ensure!(
            self.namespace
                .read_json_with_identity::<Record>(&self.path)?
                == (self.record.clone(), self.file),
            INVALID_CUSTODY
        );
        self.namespace.verify()
    }
    pub fn request_termination(&self, store: &BrokerSupervisionStore) -> Result<()> {
        self.verify()?;
        let name = format!(
            "{RECORD_PREFIX}{}{TERMINATION_SUFFIX}",
            self.record.launch_id
        );
        store.write(
            &name,
            &Termination {
                version: self.record.version,
                launch_id: &self.record.launch_id,
                supervisor: self.record.supervisor,
            },
        )
    }
}

#[derive(Serialize)]
struct Termination<'a> {
    version: u16,
    launch_id: &'a str,
    supervisor: Identity,
}
const RUNNING: &str = "running";
const RECORD_NAMESPACE: &str = "fixture-broker-supervision";
