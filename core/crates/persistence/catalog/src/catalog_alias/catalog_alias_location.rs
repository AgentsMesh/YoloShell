use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::Result;

pub(super) fn root(main: &PrivateFileEntry) -> Result<PrivateDirectory> {
    let root = crate::coordination::private_root()?;
    #[cfg(target_os = "linux")]
    if device(root.descriptor())? != device(main.parent_descriptor())? {
        return linux_root(main);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = main;
    Ok(root)
}

#[cfg(target_os = "linux")]
const LOCAL_ROOT_NAME: &str = ".yolo-catalog-coordination";
#[cfg(target_os = "linux")]
const MAX_ANCESTOR_INSPECTIONS: usize = 32;
#[cfg(target_os = "linux")]
const CATALOG_PARENT_MISSING: &str = "catalog has no parent directory";
#[cfg(target_os = "linux")]
const ALIAS_FILESYSTEM_CHANGED: &str = "catalog alias directory filesystem changed";

#[cfg(target_os = "linux")]
fn linux_root(main: &PrivateFileEntry) -> Result<PrivateDirectory> {
    use anyhow::Context;
    use std::os::unix::fs::MetadataExt;
    let expected = device(main.parent_descriptor())?;
    let parent = main.path().parent().context(CATALOG_PARENT_MISSING)?;
    // Place aliases outside the replaceable catalog directory, while remaining
    // on its filesystem. Inspect only ancestors, never other users' directories.
    for ancestor in parent.ancestors().skip(1).take(MAX_ANCESTOR_INSPECTIONS) {
        let metadata = std::fs::symlink_metadata(ancestor)?;
        if metadata.dev() != expected {
            break;
        }
        if metadata.is_dir() && metadata.uid() == unsafe { libc::geteuid() } {
            return create_root(ancestor, expected);
        }
    }
    // A catalog at a mount root has no same-filesystem ancestor. Its private
    // parent is then the only eligible location; path replacement fails closed.
    let reopened = PrivateDirectory::open(parent)?;
    anyhow::ensure!(
        identity(reopened.descriptor())? == identity(main.parent_descriptor())?,
        ALIAS_FILESYSTEM_CHANGED
    );
    create_root(parent, expected)
}

#[cfg(target_os = "linux")]
fn create_root(parent: &std::path::Path, expected: libc::dev_t) -> Result<PrivateDirectory> {
    use std::os::unix::fs::DirBuilderExt;
    let path = parent.join(LOCAL_ROOT_NAME);
    if let Err(error) = std::fs::DirBuilder::new()
        .mode(super::secure_vfs_alias_io::ALIAS_ACTIVE_MODE)
        .create(&path)
        && error.kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(error.into());
    }
    let root = PrivateDirectory::open(&path)?;
    anyhow::ensure!(
        device(root.descriptor())? == expected,
        ALIAS_FILESYSTEM_CHANGED
    );
    Ok(root)
}

#[cfg(target_os = "linux")]
fn device(descriptor: std::os::fd::RawFd) -> Result<libc::dev_t> {
    identity(descriptor).map(|identity| identity.0)
}

#[cfg(target_os = "linux")]
fn identity(descriptor: std::os::fd::RawFd) -> Result<(libc::dev_t, libc::ino_t)> {
    use anyhow::Context;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(descriptor, stat.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error()).context("inspect catalog alias filesystem");
    }
    let stat = unsafe { stat.assume_init() };
    Ok((stat.st_dev, stat.st_ino))
}
