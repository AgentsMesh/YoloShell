use super::{Observation, kind};
use crate::{
    DurableObjectIdentity, Error,
    model::{Birth, Proof},
};
use std::os::fd::{AsRawFd, BorrowedFd};

mod attributes;

const APFS_NAME: &[u8] = b"apfs\0";

pub(super) fn capture(descriptor: BorrowedFd<'_>) -> Result<DurableObjectIdentity, Error> {
    require_apfs(descriptor)?;
    let before = observe(descriptor)?;
    let volume_uuid = attributes::volume_uuid(descriptor)?;
    let (file_id, birth) = attributes::object(descriptor)?;
    before.verify(observe(descriptor)?)?;
    verify_attributes(before, file_id, birth)?;
    DurableObjectIdentity::new(
        before.kind,
        birth,
        Proof::Macos {
            volume_uuid,
            file_id,
        },
    )
}

fn verify_attributes(before: Observation, file_id: u64, birth: Birth) -> Result<(), Error> {
    if file_id != before.inode || birth != before.birth {
        return Err(Error::Changed);
    }
    Ok(())
}

fn require_apfs(descriptor: BorrowedFd<'_>) -> Result<(), Error> {
    let mut output = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: fstatfs writes one output structure for the retained descriptor.
    if unsafe { libc::fstatfs(descriptor.as_raw_fd(), output.as_mut_ptr()) }
        != libc::c_int::default()
    {
        return Err(Error::system("inspect durable filesystem"));
    }
    // SAFETY: successful fstatfs initialized the output.
    let output = unsafe { output.assume_init() };
    validate_filesystem(&output)
}

fn validate_filesystem(output: &libc::statfs) -> Result<(), Error> {
    if !output
        .f_fstypename
        .iter()
        .take(APFS_NAME.len())
        .map(|byte| *byte as u8)
        .eq(APFS_NAME.iter().copied())
    {
        return Err(Error::Unsupported("Mac filesystem is not APFS"));
    }
    Ok(())
}

fn observe(descriptor: BorrowedFd<'_>) -> Result<Observation, Error> {
    let mut output = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: fstat writes one stat for this borrowed descriptor.
    if unsafe { libc::fstat(descriptor.as_raw_fd(), output.as_mut_ptr()) } != libc::c_int::default()
    {
        return Err(Error::system("inspect durable descriptor"));
    }
    // SAFETY: successful fstat initialized the output.
    let output = unsafe { output.assume_init() };
    parse_observation(&output)
}

fn parse_observation(output: &libc::stat) -> Result<Observation, Error> {
    let nanoseconds =
        u32::try_from(output.st_birthtime_nsec).map_err(|_| Error::Invalid("birth nanoseconds"))?;
    Ok(Observation {
        device: output.st_dev as u64,
        inode: output.st_ino,
        kind: kind(u32::from(output.st_mode))?,
        birth: Birth {
            seconds: output.st_birthtime,
            nanoseconds,
        },
    })
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/macos_observation.rs"]
mod unit_tests;
