use super::*;
use crate::paths::directory_identity::DirectoryIdentity;
use anyhow::{Context, Result};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};

#[path = "isolated_test_socket_root_creation_rollback.rs"]
mod creation_rollback;

const CREATE_CONTEXT: &str = "create isolated test socket root";
const OPEN_PARENT_CONTEXT: &str = "open isolated test socket parent";
const OPEN_ROOT_CONTEXT: &str = "open isolated test socket root";
const COLLISION_ERROR: &str = "isolated test socket root collision budget exhausted";

pub(super) fn create() -> Result<IsolatedTestSocketRoot> {
    let (parent, parent_identity) = open_parent()?;
    for _ in 0..ISOLATED_TEST_SOCKET_CREATION_ATTEMPTS {
        let name = random_name(ISOLATED_TEST_SOCKET_PREFIX)?;
        let path = Path::new(ISOLATED_TEST_SOCKET_PARENT)
            .join(std::ffi::OsString::from_vec(name.to_bytes().to_vec()));
        anyhow::ensure!(
            path.as_os_str().as_bytes().len() <= ISOLATED_TEST_SOCKET_ROOT_MAX_BYTES,
            "isolated test socket root exceeds endpoint budget"
        );
        if !create_directory(&parent, &name)? {
            continue;
        }
        let mut rollback = creation_rollback::CreationRollback::capture(&parent, &name)?;
        let completed = complete_creation(&parent, &name, &path, parent_identity, &rollback);
        return match completed {
            Ok(root) => {
                rollback.disarm();
                Ok(root)
            }
            Err(error) => rollback.fail(error),
        };
    }
    anyhow::bail!(COLLISION_ERROR)
}

pub(super) fn create_directory(parent: &File, name: &CString) -> Result<bool> {
    let mode = libc::mode_t::try_from(PRIVATE_DIRECTORY_MODE)?;
    let result = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), mode) };
    if result == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        Ok(false)
    } else {
        Err(error).context(CREATE_CONTEXT)
    }
}

fn complete_creation(
    parent: &File,
    name: &CString,
    path: &Path,
    parent_identity: ObjectIdentity,
    rollback: &creation_rollback::CreationRollback<'_>,
) -> Result<IsolatedTestSocketRoot> {
    let anchor = open_child(parent, name).context(OPEN_ROOT_CONTEXT)?;
    rollback.verify_opened(&anchor)?;
    hooks::creation_after_mkdir(path)?;
    secure_root(&anchor, path)?;
    hooks::creation_before_open(path)?;
    let opened = open_child(parent, name).context(OPEN_ROOT_CONTEXT)?;
    rollback.verify_opened(&opened)?;
    hooks::creation_after_open(path)?;
    secure_root(&opened, path)?;
    rollback.verify_named()?;
    opened
        .sync_all()
        .context("sync isolated test socket root")?;
    parent
        .sync_all()
        .context("sync isolated test socket parent")?;
    let root_identity = ObjectIdentity::from_file(&opened)?
        .context("isolated socket root birth identity is unavailable")?;
    let root = PrivateDirectory::from_opened(opened, path)?;
    let mut result = IsolatedTestSocketRoot {
        parent: parent.try_clone()?,
        parent_identity,
        root,
        root_identity,
        active_name: name.clone(),
        active_path: path.to_path_buf(),
        name: name.clone(),
        path: path.to_path_buf(),
        stage: IsolatedTestSocketRootStage::Active,
        removal_synced: false,
        finalization_attempted: false,
    };
    anyhow::ensure!(
        rebind_current(&mut result)?,
        "isolated test socket root changed during creation"
    );
    Ok(result)
}

pub(super) fn verify(root: &mut IsolatedTestSocketRoot) -> Result<()> {
    let (_, parent_identity) = open_parent()?;
    anyhow::ensure!(
        parent_identity == root.parent_identity,
        "socket parent changed"
    );
    anyhow::ensure!(
        rebind_current(root)?,
        "isolated test socket root identity changed"
    );
    Ok(())
}

pub(super) fn rebind_current(root: &mut IsolatedTestSocketRoot) -> Result<bool> {
    let current = open_child(&root.parent, &root.name)?;
    crate::private_acl::require_empty(&current, &root.path)?;
    DirectoryIdentity::inspect_private(&current.metadata()?, &root.path)?;
    if ObjectIdentity::from_file(&current)? != Some(root.root_identity) {
        return Ok(false);
    }
    root.root = PrivateDirectory::from_opened(current, &root.path)?;
    Ok(true)
}

pub(super) fn random_name(prefix: &str) -> Result<CString> {
    if let Some(name) = hooks::generated_name(prefix)? {
        return Ok(name);
    }
    let encoded = uuid::Uuid::new_v4().simple().to_string();
    CString::new(format!(
        "{prefix}{}",
        &encoded[..ISOLATED_TEST_SOCKET_TOKEN_HEX_LENGTH]
    ))
    .context("isolated test socket root name contains null")
}

pub(super) fn open_child(parent: &File, name: &CString) -> std::io::Result<File> {
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

fn open_parent() -> Result<(File, ObjectIdentity)> {
    open_parent_at(Path::new(ISOLATED_TEST_SOCKET_PARENT))
}

pub(super) fn open_parent_at(path: &Path) -> Result<(File, ObjectIdentity)> {
    let name = CString::new(path.as_os_str().as_bytes())?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let descriptor = unsafe { libc::open(name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context(OPEN_PARENT_CONTEXT);
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    crate::private_acl::require_empty(&file, path)?;
    DirectoryIdentity::inspect_ancestor(&file.metadata()?, path)?;
    let identity = ObjectIdentity::from_file(&file)?
        .context("isolated socket parent birth identity is unavailable")?;
    Ok((file, identity))
}

fn secure_root(file: &File, path: &Path) -> Result<()> {
    let mode = libc::mode_t::try_from(PRIVATE_DIRECTORY_MODE)?;
    let result = unsafe { libc::fchmod(file.as_raw_fd(), mode) };
    anyhow::ensure!(result == 0, std::io::Error::last_os_error());
    crate::private_acl::clear(file, path)?;
    DirectoryIdentity::inspect_private(&file.metadata()?, path).map(|_| ())
}
