use super::super::bootstrap_exec_observer::current_exec_thread_id;
use crate::process::ProcessIdentity;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[cfg(test)]
pub(crate) mod field {
    pub(crate) use crate::protocol::field::EXEC_THREAD_ID;
    pub(crate) const TOKEN: &str = "token";
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BootstrapChildSpec {
    pub socket_path: PathBuf,
    #[serde(with = "crate::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub worker_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub token: Uuid,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BootstrapHello {
    #[serde(with = "crate::serde_uuid")]
    pub(crate) terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) worker_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) token: Uuid,
    pub(crate) exec_thread_id: u32,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BootstrapCapabilityAck {
    pub(crate) reader_handle: u64,
    pub(crate) writer_handle: u64,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct BootstrapAuthentication {
    #[serde(with = "crate::serde_uuid")]
    pub(crate) terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) worker_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) token: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum BootstrapGateReply {
    Prepared,
    Go,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BootstrapExecFailureFrame {
    pub(crate) detail: String,
}

pub(crate) struct BootstrapGateExpected {
    pub(crate) authentication: BootstrapAuthentication,
    pub(crate) child: ProcessIdentity,
}

impl BootstrapChildSpec {
    pub(crate) fn expected(&self, child: ProcessIdentity) -> BootstrapGateExpected {
        BootstrapGateExpected {
            authentication: self.authentication(),
            child,
        }
    }

    pub(crate) fn hello(&self) -> Result<BootstrapHello> {
        Ok(BootstrapHello {
            terminal_id: self.terminal_id,
            runtime_epoch: self.runtime_epoch,
            worker_id: self.worker_id,
            token: self.token,
            exec_thread_id: current_exec_thread_id()?,
        })
    }

    pub fn authentication(&self) -> BootstrapAuthentication {
        BootstrapAuthentication {
            terminal_id: self.terminal_id,
            runtime_epoch: self.runtime_epoch,
            worker_id: self.worker_id,
            token: self.token,
        }
    }
}

impl BootstrapHello {
    pub fn authentication(&self) -> BootstrapAuthentication {
        BootstrapAuthentication {
            terminal_id: self.terminal_id,
            runtime_epoch: self.runtime_epoch,
            worker_id: self.worker_id,
            token: self.token,
        }
    }
}
