use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use yolo_hub_identity::HubId;
use yolo_paths::directory::PrivateDirectory;
use zeroize::Zeroize;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema: u16,
    pub hub_id: HubId,
    pub binding: Binding,
    pub host_home: PathBuf,
    pub key: Vec<u8>,
}

impl Drop for Record {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

#[derive(Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Binding {
    path: PathBuf,
    directory: yolo_paths::identity::DurableObjectIdentity,
    user: u32,
}

impl Binding {
    pub fn capture(directory: &PrivateDirectory) -> Result<Self> {
        Ok(Self {
            path: directory.path().to_path_buf(),
            directory: directory.durable_identity()?,
            user: unsafe { libc::geteuid() },
        })
    }
}

impl Record {
    pub fn verify(&self, binding: &Binding, host_home: &PathBuf) -> Result<()> {
        ensure!(self.schema == STATE_SCHEMA_VERSION, INVALID_SCHEMA);
        ensure!(&self.binding == binding, CLONED_STATE);
        ensure!(&self.host_home == host_home, WRONG_BACKEND);
        Ok(())
    }
}
