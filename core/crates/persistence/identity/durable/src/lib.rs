//! Descriptor-based identity for reopening the same filesystem object.
//!
//! This does not establish pathname ownership, permissions, content versions,
//! lock ownership, or authority over a process. Keep live descriptor fencing.
mod error;
mod model;
mod platform;

pub use error::Error;
pub use model::{DurableObjectIdentity, ObjectKind};

impl DurableObjectIdentity {
    pub fn capture(descriptor: std::os::fd::BorrowedFd<'_>) -> Result<Self, Error> {
        platform::capture(descriptor)
    }

    pub fn verify(&self, descriptor: std::os::fd::BorrowedFd<'_>) -> Result<(), Error> {
        if Self::capture(descriptor)? == *self {
            Ok(())
        } else {
            Err(Error::Changed)
        }
    }

    pub fn kind(&self) -> ObjectKind {
        self.record.kind
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/mod.rs"]
mod integration_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/mod.rs"]
mod unit_tests;
