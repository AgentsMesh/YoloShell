use crate::{HubBrokerLaunchScope, constants::*};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_process::ProcessIdentity;
use yolo_remote_broker::BrokerStatus;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChildIdentity {
    pub pid: u32,
    pub start_time: u64,
}
impl From<ProcessIdentity> for ChildIdentity {
    fn from(value: ProcessIdentity) -> Self {
        Self {
            pid: value.pid,
            start_time: value.start_time,
        }
    }
}
impl From<ChildIdentity> for ProcessIdentity {
    fn from(value: ChildIdentity) -> Self {
        Self {
            pid: value.pid,
            start_time: value.start_time,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Action {
    Hello {},
    Verify {},
    Commit { ready: BrokerStatus },
    Finished { ready: BrokerStatus },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub version: u16,
    pub child: ChildIdentity,
    pub nonce: Uuid,
    pub sequence: u64,
    pub action: Action,
}

impl Request {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.version == VERSION
                && !self.nonce.is_nil()
                && self.child.pid != u32::default()
                && self.child.start_time != u64::default(),
            INVALID
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Statement {
    pub scope: HubBrokerLaunchScope,
    pub request: Request,
    pub credentials_sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignedStatement {
    pub statement: Statement,
    pub signature: Vec<u8>,
}
