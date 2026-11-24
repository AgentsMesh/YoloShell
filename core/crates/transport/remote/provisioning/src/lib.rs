mod command;
mod connection;
mod constants;
mod journal;
mod owner;
mod record;
mod recovery;
mod service;
use anyhow::{Result, ensure};
pub use constants::PROVISIONING_DIRECTORY;
use yolo_broker_hub_protocol::BrokerHubProfile;
use yolo_paths::directory::PrivateDirectory;
use yolo_remote_provisioning_protocol::ProvisionRequest;

/// Implemented by broker admission; payloads cannot create it or acquire SSH credentials.
pub trait ProvisionGuard: Send + Sync {
    fn verify(&self) -> Result<()>;
}
impl<F: Fn() -> Result<()> + Send + Sync> ProvisionGuard for F {
    fn verify(&self) -> Result<()> {
        self()
    }
}

pub struct RemoteProvisioningService {
    parent: PrivateDirectory,
    root: PrivateDirectory,
    profile: BrokerHubProfile,
}
impl RemoteProvisioningService {
    pub fn open_existing(
        parent: PrivateDirectory,
        profile: BrokerHubProfile,
    ) -> Result<Option<Self>> {
        profile.validate()?;
        parent.verify_path()?;
        let root = match parent.open_descendant(&parent.path().join(PROVISIONING_DIRECTORY)) {
            Ok(root) => root,
            Err(error)
                if yolo_paths::private_file::io_kind(&error)
                    == Some(std::io::ErrorKind::NotFound) =>
            {
                parent.verify_path()?;
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        Ok(Some(Self {
            parent,
            root,
            profile,
        }))
    }
    pub fn open(parent: PrivateDirectory, profile: BrokerHubProfile) -> Result<Self> {
        profile.validate()?;
        parent.verify_path()?;
        let root = parent.ensure_descendant(&parent.path().join(PROVISIONING_DIRECTORY))?;
        root.sync()?;
        parent.sync()?;
        Ok(Self {
            parent,
            root,
            profile,
        })
    }
    fn verify(&self, request: &ProvisionRequest) -> Result<()> {
        ensure!(
            request.fields().profile == self.profile,
            constants::CONFLICT
        );
        self.parent.verify_path()?;
        self.root.verify_path()
    }
}
