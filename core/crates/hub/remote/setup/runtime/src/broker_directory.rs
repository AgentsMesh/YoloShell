use crate::constants::JOURNAL_CHANGED;
use anyhow::{Result, ensure};
use yolo_hub_ssh_profile_protocol::SshBrokerProfile;
use yolo_paths::{directory::PrivateDirectory, identity::DurableObjectIdentity};

/// Persistent continuity is checked at admission; the retained descriptor fences
/// every later await against replacement or remount during this operation.
pub(crate) struct BrokerDirectory {
    directory: PrivateDirectory,
    durable: DurableObjectIdentity,
}

impl BrokerDirectory {
    pub fn open(
        registration: &SshBrokerProfile,
        expected: Option<DurableObjectIdentity>,
    ) -> Result<Self> {
        let directory = PrivateDirectory::open(&registration.broker_state)?;
        let durable = directory.durable_identity()?;
        ensure!(
            expected.is_none_or(|expected| expected == durable),
            JOURNAL_CHANGED
        );
        Ok(Self { directory, durable })
    }

    pub fn identity(&self) -> DurableObjectIdentity {
        self.durable
    }

    pub fn verify(&self) -> Result<()> {
        self.directory.verify_path()
    }
}
