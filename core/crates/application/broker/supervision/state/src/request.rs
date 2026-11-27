use crate::{
    ChildLaunch,
    constants::*,
    record::{PersistedIdentity, SupervisionRecord},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_remote_broker::BrokerSupervisionStore;

const RECORD_BROKER_DETAIL: &str = "broker supervision child identity mismatch";
const RECORD_INSTANCE_DETAIL: &str = "broker supervision instance identity mismatch";
const RECORD_GENERATION_DETAIL: &str = "broker supervision request generation mismatch";
const RECORD_HOST_DETAIL: &str = "broker supervision request host identity mismatch";
const ADOPTION_RECORD_INCOMPLETE: &str = "broker supervision adoption record is incomplete";

#[derive(Clone, Debug)]
pub struct SupervisionPaths {
    store: BrokerSupervisionStore,
    record: String,
    termination: String,
    adoption: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CustodyRequest {
    version: u16,
    launch_id: String,
    supervisor: PersistedIdentity,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AdoptionRequest {
    version: u16,
    launch_id: String,
    supervisor: PersistedIdentity,
    broker: PersistedIdentity,
    instance_id: uuid::Uuid,
    generation: String,
    host_id: String,
}

impl SupervisionPaths {
    pub fn for_launch(options: &ChildLaunch) -> Result<Self> {
        let store =
            BrokerSupervisionStore::open_expected(options.state_dir(), options.state_anchor())?;
        crate::scavenge::run(&store)?;
        let launch_id = crate::launch_id(options);
        let stem = format!("{RECORD_PREFIX}{launch_id}");
        Ok(Self {
            store,
            record: format!("{stem}{RECORD_SUFFIX}"),
            termination: format!("{stem}{TERMINATION_SUFFIX}"),
            adoption: format!("{stem}{ADOPTION_SUFFIX}"),
        })
    }

    pub fn write_record(&self, record: &SupervisionRecord) -> Result<()> {
        self.store.write(&self.record, record)
    }

    pub fn read_record(&self) -> Result<SupervisionRecord> {
        self.store.read(&self.record)
    }

    pub fn contain_broker(&self, record: &SupervisionRecord) -> Result<()> {
        self.store
            .contain_recorded_broker(&self.record, record.launch_id.parse()?)
    }

    pub fn write_adoption(&self, record: &SupervisionRecord) -> Result<()> {
        self.store
            .write(&self.adoption, &AdoptionRequest::for_record(record)?)
    }

    pub fn write_termination(&self, record: &SupervisionRecord) -> Result<()> {
        self.store
            .write(&self.termination, &CustodyRequest::for_record(record))
    }

    pub fn adoption_present(&self, record: &SupervisionRecord) -> Result<bool> {
        match self.store.read::<AdoptionRequest>(&self.adoption) {
            Ok(request) => request.verify(record).map(|_| true),
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn termination_present(&self, record: &SupervisionRecord) -> Result<bool> {
        self.request_present(&self.termination, record)
    }

    pub fn remove_adoption(&self) -> Result<()> {
        self.store.remove(&self.adoption)
    }

    pub fn remove_requests(&self) -> Result<()> {
        let adoption = self.store.remove(&self.adoption);
        let termination = self.store.remove(&self.termination);
        adoption.and(termination)
    }

    pub fn remove_all(&self) -> Result<()> {
        self.remove_requests()?;
        self.store.remove(&self.record)
    }

    fn request_present(&self, name: &str, record: &SupervisionRecord) -> Result<bool> {
        match self.store.read::<CustodyRequest>(name) {
            Ok(request) => request.verify(record).map(|_| true),
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

impl AdoptionRequest {
    fn for_record(record: &SupervisionRecord) -> Result<Self> {
        Ok(Self {
            version: RECORD_VERSION,
            launch_id: record.launch_id.clone(),
            supervisor: record.supervisor,
            broker: record.broker.context(ADOPTION_RECORD_INCOMPLETE)?,
            instance_id: record.instance_id.context(ADOPTION_RECORD_INCOMPLETE)?,
            generation: record.generation.clone(),
            host_id: record
                .broker_host_id
                .clone()
                .context(ADOPTION_RECORD_INCOMPLETE)?,
        })
    }

    fn verify(&self, record: &SupervisionRecord) -> Result<()> {
        ensure!(self.version == RECORD_VERSION, RECORD_VERSION_DETAIL);
        ensure!(self.launch_id == record.launch_id, RECORD_LAUNCH_DETAIL);
        ensure!(
            self.supervisor == record.supervisor,
            RECORD_SUPERVISOR_DETAIL
        );
        ensure!(Some(self.broker) == record.broker, RECORD_BROKER_DETAIL);
        ensure!(
            Some(self.instance_id) == record.instance_id,
            RECORD_INSTANCE_DETAIL
        );
        ensure!(
            self.generation == record.generation,
            RECORD_GENERATION_DETAIL
        );
        ensure!(
            Some(self.host_id.as_str()) == record.broker_host_id.as_deref(),
            RECORD_HOST_DETAIL
        );
        Ok(())
    }
}

impl CustodyRequest {
    pub fn for_record(record: &SupervisionRecord) -> Self {
        Self {
            version: RECORD_VERSION,
            launch_id: record.launch_id.clone(),
            supervisor: record.supervisor,
        }
    }

    pub fn verify(&self, record: &SupervisionRecord) -> Result<()> {
        ensure!(self.version == RECORD_VERSION, RECORD_VERSION_DETAIL);
        ensure!(self.launch_id == record.launch_id, RECORD_LAUNCH_DETAIL);
        ensure!(
            self.supervisor == record.supervisor,
            RECORD_SUPERVISOR_DETAIL
        );
        Ok(())
    }
}

pub fn io_kind(error: &anyhow::Error) -> Option<std::io::ErrorKind> {
    error.chain().find_map(|source| {
        source
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind)
    })
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/request.rs"]
mod tests;
