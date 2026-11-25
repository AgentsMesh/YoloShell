use crate::{DurableObjectIdentity, Error, ObjectKind, model::Birth};
use std::os::fd::BorrowedFd;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[derive(Clone, Copy, Eq, PartialEq)]
struct Observation {
    device: u64,
    inode: u64,
    kind: ObjectKind,
    birth: Birth,
}

impl Observation {
    fn verify(self, after: Self) -> Result<(), Error> {
        if self == after {
            Ok(())
        } else {
            Err(Error::Changed)
        }
    }
}

pub(crate) fn capture(descriptor: BorrowedFd<'_>) -> Result<DurableObjectIdentity, Error> {
    #[cfg(target_os = "linux")]
    {
        linux::capture(descriptor)
    }
    #[cfg(target_os = "macos")]
    {
        macos::capture(descriptor)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = descriptor;
        Err(Error::Unsupported("platform"))
    }
}

fn kind(mode: u32) -> Result<ObjectKind, Error> {
    match mode & u32::from(libc::S_IFMT) {
        value if value == u32::from(libc::S_IFDIR) => Ok(ObjectKind::Directory),
        value if value == u32::from(libc::S_IFREG) => Ok(ObjectKind::RegularFile),
        _ => Err(Error::Unsupported(
            "object is not a directory or regular file",
        )),
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/observation.rs"]
mod observation_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/object_kind.rs"]
mod unit_tests;
