mod claim;
mod constants;
mod model;
mod observation;
mod record;
mod startup;

use anyhow::Result;
pub use claim::{Admission, SubmitPermit};
pub use constants::ENROLLMENT_DIRECTORY;
pub use model::{EnrollmentRecord, PreparedEnrollment};
pub use observation::Observation;
pub use startup::{StartupAdmission, admit_startup};
use yolo_hub_delegation::PeerIdentity;
use yolo_remote_provisioning_protocol::RemoteBootstrapRequest;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, private_file::io_kind,
};

/// A process-local request owner. Neither this nor SubmitPermit can be deserialized.
pub struct Claim {
    parent: PrivateDirectory,
    root: PrivateDirectory,
    lock: CoordinationLock,
    peer: PeerIdentity,
    request: RemoteBootstrapRequest,
}

pub fn claim(
    parent: PrivateDirectory,
    peer: PeerIdentity,
    request: &RemoteBootstrapRequest,
) -> Result<Admission> {
    claim::claim(parent, peer, request)
}

/// Does not create directories/locks or repair incomplete entries.
pub fn inspect(
    parent: &PrivateDirectory,
    peer: PeerIdentity,
    request: &RemoteBootstrapRequest,
) -> Result<Option<Observation>> {
    parent.verify_path()?;
    let root = match parent.open_descendant(&parent.path().join(ENROLLMENT_DIRECTORY)) {
        Ok(root) => root,
        Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
            parent.verify_path()?;
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    observation::inspect(parent, &root, peer, request)
}
