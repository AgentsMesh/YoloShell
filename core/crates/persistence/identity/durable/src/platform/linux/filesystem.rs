use crate::{
    Error,
    model::{BTRFS_MAGIC, UUID_BYTES, supported_linux_filesystem},
};
use std::os::fd::{AsRawFd, BorrowedFd};

const BTRFS_IOCTL_GROUP: u32 = 0x94;
const BTRFS_FS_INFO_COMMAND: u32 = 31;
const BTRFS_FS_INFO_RESERVED_BYTES: usize = 944;
const FILESYSTEM_IOCTL_GROUP: u32 = 0x15;
const FILESYSTEM_UUID_COMMAND: u32 = 0;

#[repr(C)]
struct BtrfsInfo {
    max_id: u64,
    num_devices: u64,
    uuid: [u8; UUID_BYTES],
    node_size: u32,
    sector_size: u32,
    clone_alignment: u32,
    checksum_type: u16,
    checksum_size: u16,
    flags: u64,
    generation: u64,
    metadata_uuid: [u8; UUID_BYTES],
    reserved: [u8; BTRFS_FS_INFO_RESERVED_BYTES],
}

#[repr(C)]
struct FilesystemUuid {
    length: u8,
    uuid: [u8; UUID_BYTES],
}

pub(super) fn capture(descriptor: BorrowedFd<'_>) -> Result<(u32, [u8; UUID_BYTES]), Error> {
    let mut filesystem = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: fstatfs writes one statfs into the output for this borrowed FD.
    if unsafe { libc::fstatfs(descriptor.as_raw_fd(), filesystem.as_mut_ptr()) }
        != libc::c_int::default()
    {
        return Err(Error::system("inspect durable filesystem"));
    }
    // SAFETY: the successful call initialized statfs.
    let filesystem = unsafe { filesystem.assume_init() };
    let kind =
        u32::try_from(filesystem.f_type).map_err(|_| Error::Unsupported("filesystem type"))?;
    if !supported_linux_filesystem(kind) {
        return Err(Error::Unsupported(
            "filesystem has no supported durable identity backend",
        ));
    }
    let uuid = if kind == BTRFS_MAGIC {
        btrfs_uuid(descriptor)?
    } else {
        generic_uuid(descriptor)?
    };
    Ok((kind, uuid))
}

fn btrfs_uuid(descriptor: BorrowedFd<'_>) -> Result<[u8; UUID_BYTES], Error> {
    // SAFETY: all fields are integers/bytes and zero is valid for the request.
    let mut output: BtrfsInfo = unsafe { std::mem::zeroed() };
    let request = libc::_IOR::<BtrfsInfo>(BTRFS_IOCTL_GROUP, BTRFS_FS_INFO_COMMAND);
    // SAFETY: ioctl's request encodes the exact repr(C) output structure size.
    let status = unsafe { libc::ioctl(descriptor.as_raw_fd(), request, &mut output) };
    require_uuid_support(status)?;
    Ok(output.uuid)
}

fn generic_uuid(descriptor: BorrowedFd<'_>) -> Result<[u8; UUID_BYTES], Error> {
    let mut output = FilesystemUuid {
        length: u8::default(),
        uuid: [u8::default(); UUID_BYTES],
    };
    let request = libc::_IOR::<FilesystemUuid>(FILESYSTEM_IOCTL_GROUP, FILESYSTEM_UUID_COMMAND);
    // SAFETY: ioctl's request encodes the exact repr(C) output structure size.
    let status = unsafe { libc::ioctl(descriptor.as_raw_fd(), request, &mut output) };
    require_uuid_support(status)?;
    parse_uuid(output)
}

fn parse_uuid(output: FilesystemUuid) -> Result<[u8; UUID_BYTES], Error> {
    if usize::from(output.length) != UUID_BYTES {
        return Err(Error::Unsupported("filesystem UUID length"));
    }
    Ok(output.uuid)
}

fn require_uuid_support(status: libc::c_int) -> Result<(), Error> {
    if status == libc::c_int::default() {
        return Ok(());
    }
    let source = std::io::Error::last_os_error();
    if matches!(
        source.raw_os_error(),
        Some(libc::ENOTTY | libc::EOPNOTSUPP | libc::ENOSYS)
    ) {
        return Err(Error::Unsupported("filesystem UUID"));
    }
    Err(Error::Io {
        operation: "read filesystem UUID",
        source,
    })
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/linux_filesystem.rs"]
mod integration_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/linux_filesystem.rs"]
mod unit_tests;
