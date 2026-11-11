use super::{CgroupDirectoryIdentity, LinuxContainmentError, LinuxContainmentRecord, descriptor};
use crate::process::ProcessIdentity;
use std::fs::File;
use std::os::fd::{AsRawFd, RawFd};
use std::path::Component;
use uuid::Uuid;

const BOUND_ROOT_DETAIL: &str = "containment custody root identity mismatch";
const CLOSE_ON_EXEC_DETAIL: &str = "containment custody descriptor is inheritable";
const FILESYSTEM_DETAIL: &str = "containment custody descriptor is not cgroup v2";
const LEAF_LINK_DETAIL: &str = "containment custody leaf is not bound to its recorded name";
const MOUNT_DETAIL: &str = "containment custody descriptors are on different mounts";
const READ_ONLY_DETAIL: &str = "containment custody descriptor is not read-only";
const ROOT_DESCRIPTOR_DETAIL: &str = "containment custody root descriptor identity mismatch";
const ROOT_PATH_DETAIL: &str = "containment custody delegated root is not normalized";
const LEAF_DESCRIPTOR_DETAIL: &str = "containment custody leaf descriptor identity mismatch";
const SYSTEM_FAILURE: libc::c_int = -1;

pub(super) fn validate(
    root: &File,
    leaf: &File,
    record: &LinuxContainmentRecord,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    bound_root: ProcessIdentity,
) -> Result<(CgroupDirectoryIdentity, CgroupDirectoryIdentity), LinuxContainmentError> {
    record.validate(terminal_id, runtime_epoch, &super::super::boot_id())?;
    validate_root_path(record)?;
    if record.bound_pid.zip(record.bound_start_time)
        != Some((bound_root.pid, bound_root.start_time))
    {
        return Err(LinuxContainmentError::Identity(BOUND_ROOT_DETAIL));
    }
    validate_descriptor(root)?;
    validate_descriptor(leaf)?;
    let root_identity = descriptor::identity(root)?;
    let leaf_identity = descriptor::identity(leaf)?;
    if root_identity != record.root_identity {
        return Err(LinuxContainmentError::Identity(ROOT_DESCRIPTOR_DETAIL));
    }
    if leaf_identity != record.leaf_identity {
        return Err(LinuxContainmentError::Identity(LEAF_DESCRIPTOR_DETAIL));
    }
    if root_identity.mount_id != leaf_identity.mount_id {
        return Err(LinuxContainmentError::Identity(MOUNT_DETAIL));
    }
    let named_leaf = descriptor::open_leaf(root, &record.leaf_name)?;
    if descriptor::identity(&named_leaf)? != leaf_identity {
        return Err(LinuxContainmentError::Identity(LEAF_LINK_DETAIL));
    }
    Ok((root_identity, leaf_identity))
}

fn validate_root_path(record: &LinuxContainmentRecord) -> Result<(), LinuxContainmentError> {
    let path = &record.delegated_root;
    let normalized = path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)));
    normalized
        .then_some(())
        .ok_or(LinuxContainmentError::Identity(ROOT_PATH_DETAIL))
}

fn validate_descriptor(file: &File) -> Result<(), LinuxContainmentError> {
    let descriptor = file.as_raw_fd();
    let descriptor_flags = fcntl(descriptor, libc::F_GETFD)?;
    if descriptor_flags & libc::FD_CLOEXEC == libc::c_int::default() {
        return Err(LinuxContainmentError::Identity(CLOSE_ON_EXEC_DETAIL));
    }
    let status_flags = fcntl(descriptor, libc::F_GETFL)?;
    if status_flags & libc::O_ACCMODE != libc::O_RDONLY {
        return Err(LinuxContainmentError::Identity(READ_ONLY_DETAIL));
    }
    let mut state = std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::fstatfs(descriptor, state.as_mut_ptr()) } == SYSTEM_FAILURE {
        return Err(std::io::Error::last_os_error().into());
    }
    if unsafe { state.assume_init() }.f_type != super::constants::CGROUP2_SUPER_MAGIC {
        return Err(LinuxContainmentError::Identity(FILESYSTEM_DETAIL));
    }
    Ok(())
}

fn fcntl(descriptor: RawFd, operation: libc::c_int) -> Result<libc::c_int, LinuxContainmentError> {
    let value = unsafe { libc::fcntl(descriptor, operation) };
    if value == SYSTEM_FAILURE {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(value)
    }
}
