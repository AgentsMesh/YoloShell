use crate::{Admission, Claim, constants::*, observation};
use anyhow::{Result, ensure};
use yolo_hub_delegation::PeerIdentity;
use yolo_remote_provisioning_protocol::RemoteBootstrapRequest;
use yolo_paths::{coordination::CoordinationLock, directory::PrivateDirectory};

/// Serializes one complete request before service startup. It carries no Hub identity
/// or submission permission, and is converted into Claim only after live startup.
pub struct StartupAdmission {
    parent: PrivateDirectory,
    root: PrivateDirectory,
    lock: CoordinationLock,
    request: RemoteBootstrapRequest,
}

pub fn admit_startup(
    parent: PrivateDirectory,
    request: &RemoteBootstrapRequest,
) -> Result<Option<StartupAdmission>> {
    parent.verify_path()?;
    let root = parent.ensure_descendant(&parent.path().join(ENROLLMENT_DIRECTORY))?;
    root.sync()?;
    parent.sync()?;
    let lock_path = root
        .path()
        .join(format!("{}{LOCK_SUFFIX}", request.fields().request_id));
    let Some(lock) = CoordinationLock::try_acquire(&lock_path, LOCK_BYTES)? else {
        return Ok(None);
    };
    let admission = StartupAdmission {
        parent,
        root,
        lock,
        request: request.clone(),
    };
    admission.verify()?;
    Ok(Some(admission))
}

impl StartupAdmission {
    pub fn parent_directory(&self) -> Result<PrivateDirectory> {
        self.verify()?;
        self.parent.try_clone()
    }
    pub fn verify(&self) -> Result<()> {
        self.parent.verify_path()?;
        self.root.verify_path()?;
        self.lock.verify()
    }
    pub fn claim(self, peer: PeerIdentity) -> Result<Admission> {
        self.verify()?;
        ensure!(
            self.request
                .fields()
                .expected_peer
                .is_none_or(|expected| expected == peer),
            CONFLICT
        );
        if let Some(existing) = observation::inspect(&self.parent, &self.root, peer, &self.request)?
        {
            return Ok(Admission::Existing(existing));
        }
        Ok(Admission::Fresh(Claim {
            parent: self.parent,
            root: self.root,
            lock: self.lock,
            peer,
            request: self.request,
        }))
    }
}
