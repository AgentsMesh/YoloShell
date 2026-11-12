use super::*;
use anyhow::{Context, Result};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStringExt;

#[path = "isolated_test_socket_root_settlement.rs"]
mod settlement;

pub(super) const LIFECYCLE_BUSY_ERROR: &str = "isolated test socket lifecycle lock is busy";
const UNKNOWN_ENTRY_ERROR: &str = "isolated test socket root contains an unknown entry";
const RETIRE_CONTEXT: &str = "retire isolated test socket root";

type LifecycleLock = super::super::socket_namespace::SocketNamespaceLifecycleLock;

pub(super) fn finalize(root: &mut IsolatedTestSocketRoot) -> Result<()> {
    if root.stage == IsolatedTestSocketRootStage::Removed {
        return sync_removal(root);
    }
    if root.stage == IsolatedTestSocketRootStage::Active {
        settlement::settle(root)?;
    }
    let lifecycle = match root.stage {
        IsolatedTestSocketRootStage::Active => retire_active(root)?,
        IsolatedTestSocketRootStage::Retired => acquire_verified(root)?,
        IsolatedTestSocketRootStage::Removed => unreachable!(),
    };
    create::verify(root)?;
    require_empty(&root.root)?;
    create::verify(root)?;
    hooks::retire_before_root_unlink(&root.path)?;
    remove_retired(root)?;
    drop(lifecycle);
    Ok(())
}

fn retire_active(root: &mut IsolatedTestSocketRoot) -> Result<LifecycleLock> {
    create::verify(root)?;
    let lifecycle = acquire_verified(root)?;
    require_empty(&root.root)?;
    hooks::retire_before_rename(&root.path)?;
    rename_owned(root)?;
    hooks::retire_after_rename(&root.path)?;
    root.parent
        .sync_all()
        .context("sync retired socket root rename")?;
    hooks::retire_before_open(&root.path)?;
    if !create::rebind_current(root)? {
        rollback_replacement(root)?;
        anyhow::bail!("retired isolated test socket root identity changed");
    }
    Ok(lifecycle)
}

fn acquire_verified(root: &mut IsolatedTestSocketRoot) -> Result<LifecycleLock> {
    create::verify(root)?;
    let lifecycle =
        super::super::socket_namespace::SocketNamespaceLifecycleLock::try_acquire(&root.root)?
            .ok_or_else(|| anyhow::anyhow!(LIFECYCLE_BUSY_ERROR))?;
    create::verify(root)?;
    Ok(lifecycle)
}

fn rename_owned(root: &mut IsolatedTestSocketRoot) -> Result<()> {
    for _ in 0..ISOLATED_TEST_SOCKET_CREATION_ATTEMPTS {
        let retired = create::random_name(ISOLATED_TEST_SOCKET_RETIRED_PREFIX)?;
        match super::super::private_file::rename_exclusive_between(
            root.parent.as_raw_fd(),
            &root.name,
            root.parent.as_raw_fd(),
            &retired,
        ) {
            Ok(()) => {
                root.name = retired;
                root.path = Path::new(ISOLATED_TEST_SOCKET_PARENT)
                    .join(std::ffi::OsString::from_vec(root.name.to_bytes().to_vec()));
                root.stage = IsolatedTestSocketRootStage::Retired;
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).context(RETIRE_CONTEXT),
        }
    }
    anyhow::bail!("isolated test socket retirement collision budget exhausted")
}

fn rollback_replacement(root: &mut IsolatedTestSocketRoot) -> Result<()> {
    super::super::private_file::rename_exclusive_between(
        root.parent.as_raw_fd(),
        &root.name,
        root.parent.as_raw_fd(),
        &root.active_name,
    )
    .context("restore replaced isolated test socket root")?;
    root.name = root.active_name.clone();
    root.path = root.active_path.clone();
    root.stage = IsolatedTestSocketRootStage::Active;
    root.parent
        .sync_all()
        .context("sync restored isolated test socket root")
}

fn require_empty(root: &PrivateDirectory) -> Result<()> {
    let mut stream = super::super::directory_stream::DirectoryStream::open(root.descriptor())?;
    if let Some(name) = stream.next_name()? {
        anyhow::bail!("{UNKNOWN_ENTRY_ERROR}: {name:?}");
    }
    Ok(())
}

fn remove_retired(root: &mut IsolatedTestSocketRoot) -> Result<()> {
    let opened = create::open_child(&root.parent, &root.name)?;
    let current = ObjectIdentity::from_file(&opened)?
        .context("retired isolated socket root birth identity is unavailable")?;
    anyhow::ensure!(
        current == root.root_identity,
        "retired isolated test socket root identity changed"
    );
    let result = unsafe {
        libc::unlinkat(
            root.parent.as_raw_fd(),
            root.name.as_ptr(),
            libc::AT_REMOVEDIR,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context(RETIRE_CONTEXT);
    }
    root.stage = IsolatedTestSocketRootStage::Removed;
    sync_removal(root)
}

fn sync_removal(root: &mut IsolatedTestSocketRoot) -> Result<()> {
    if root.removal_synced {
        return Ok(());
    }
    root.parent
        .sync_all()
        .context("sync isolated test root removal")?;
    root.removal_synced = true;
    Ok(())
}
