use super::{Observation, kind};
use crate::{
    DurableObjectIdentity, Error,
    model::{Birth, Proof},
};
use std::os::fd::{AsRawFd, BorrowedFd};

mod filesystem;
mod handle;

const STATX_FIELDS: u32 = libc::STATX_TYPE | libc::STATX_INO | libc::STATX_BTIME;
const NANOSECONDS_PER_SECOND: u32 = 1_000_000_000;

pub(super) fn capture(descriptor: BorrowedFd<'_>) -> Result<DurableObjectIdentity, Error> {
    let before = observe(descriptor)?;
    let (filesystem_type, filesystem_uuid) = filesystem::capture(descriptor)?;
    let handle = handle::capture(descriptor)?;
    before.verify(observe(descriptor)?)?;
    DurableObjectIdentity::new(
        before.kind,
        before.birth,
        Proof::Linux {
            filesystem_type,
            filesystem_uuid,
            handle,
        },
    )
}

fn observe(descriptor: BorrowedFd<'_>) -> Result<Observation, Error> {
    let mut output = std::mem::MaybeUninit::<libc::statx>::uninit();
    // SAFETY: statx writes the supplied initialized-size output for a borrowed FD.
    let status = unsafe {
        libc::statx(
            descriptor.as_raw_fd(),
            c"".as_ptr(),
            libc::AT_EMPTY_PATH | libc::AT_SYMLINK_NOFOLLOW,
            STATX_FIELDS,
            output.as_mut_ptr(),
        )
    };
    if status != libc::c_int::default() {
        return Err(Error::system("inspect durable descriptor"));
    }
    // SAFETY: successful statx initialized the output structure.
    let output = unsafe { output.assume_init() };
    parse_observation(&output)
}

fn parse_observation(output: &libc::statx) -> Result<Observation, Error> {
    if output.stx_mask & STATX_FIELDS != STATX_FIELDS {
        return Err(Error::Unsupported(
            "descriptor birth identity is unavailable",
        ));
    }
    if output.stx_btime.tv_nsec >= NANOSECONDS_PER_SECOND {
        return Err(Error::Invalid("birth nanoseconds"));
    }
    Ok(Observation {
        device: libc::makedev(output.stx_dev_major, output.stx_dev_minor),
        inode: output.stx_ino,
        kind: kind(u32::from(output.stx_mode))?,
        birth: Birth {
            seconds: output.stx_btime.tv_sec,
            nanoseconds: output.stx_btime.tv_nsec,
        },
    })
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/linux_observation.rs"]
mod observation_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/linux_syscalls.rs"]
mod syscall_tests;
