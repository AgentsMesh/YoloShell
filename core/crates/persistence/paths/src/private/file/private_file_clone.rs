#[cfg(target_os = "linux")]
use super::private_file_io::{open_named, read_flags, set_private_mode};
use super::*;
#[cfg(target_os = "linux")]
use crate::paths::constants::PERMISSION_MODE_MASK;
use std::ffi::CStr;
#[cfg(target_os = "linux")]
use std::io::Write;
#[cfg(target_os = "linux")]
use std::os::fd::FromRawFd;
use std::os::fd::{AsRawFd, RawFd};
#[cfg(target_os = "linux")]
use std::os::unix::fs::{FileExt, MetadataExt};

#[cfg(target_os = "macos")]
const CLONE_NO_OWNER_COPY_FLAG: u32 = 2;
#[cfg(target_os = "linux")]
const FILE_START_OFFSET: u64 = 0;
#[cfg(target_os = "linux")]
const ANONYMOUS_FILE_FLAGS: i32 =
    libc::O_TMPFILE | libc::O_RDWR | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NOCTTY;
#[cfg(target_os = "linux")]
const ANONYMOUS_FILE_LINK_COUNT: u64 = 0;

pub(super) fn clone_private_file_at(
    source: &File,
    directory: &PrivateDirectory,
    name: &CStr,
    path: &Path,
    max_bytes: u64,
) -> Result<()> {
    clone_platform_file(source, directory.descriptor(), name, path, max_bytes)?;
    run_clone_test_hook()
}

#[cfg(target_os = "macos")]
fn clone_platform_file(
    source: &File,
    parent: RawFd,
    name: &CStr,
    _path: &Path,
    _max_bytes: u64,
) -> Result<()> {
    let result = unsafe {
        libc::fclonefileat(
            source.as_raw_fd(),
            parent,
            name.as_ptr(),
            CLONE_NO_OWNER_COPY_FLAG,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context("atomically clone retained slot")
    }
}

#[cfg(target_os = "linux")]
fn clone_platform_file(
    source: &File,
    parent: RawFd,
    name: &CStr,
    path: &Path,
    max_bytes: u64,
) -> Result<()> {
    let mode = libc::mode_t::try_from(PRIVATE_FILE_MODE)?;
    let descriptor = unsafe { libc::openat(parent, c".".as_ptr(), ANONYMOUS_FILE_FLAGS, mode) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("create anonymous retained slot");
    }
    let mut temporary = unsafe { File::from_raw_fd(descriptor) };
    set_private_mode(&temporary, path)?;
    copy_bounded(source, &mut temporary, path, max_bytes)?;
    temporary
        .sync_all()
        .context("sync anonymous retained slot")?;
    let result = unsafe {
        libc::linkat(
            temporary.as_raw_fd(),
            c"".as_ptr(),
            parent,
            name.as_ptr(),
            libc::AT_EMPTY_PATH,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context("publish anonymous retained slot");
    }
    verify_linked_clone(&temporary, parent, name, path, max_bytes)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn clone_platform_file(
    _source: &File,
    _parent: RawFd,
    _name: &CStr,
    _path: &Path,
    _max_bytes: u64,
) -> Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "atomic retained slot creation is unsupported",
    )
    .into())
}

#[cfg(target_os = "linux")]
fn copy_bounded(source: &File, target: &mut File, path: &Path, max_bytes: u64) -> Result<()> {
    let length = source.metadata()?.len();
    anyhow::ensure!(length <= max_bytes, "retained clone exceeds byte limit");
    let mut bytes = vec![0_u8; usize::try_from(length)?];
    source.read_exact_at(&mut bytes, FILE_START_OFFSET)?;
    target.write_all(&bytes)?;
    verify_anonymous_clone(target, path, length, max_bytes)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn verify_anonymous_clone(file: &File, path: &Path, length: u64, max_bytes: u64) -> Result<()> {
    let metadata = file.metadata()?;
    anyhow::ensure!(
        metadata.is_file(),
        "anonymous retained clone is not regular"
    );
    anyhow::ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "anonymous retained clone owner mismatch"
    );
    anyhow::ensure!(
        metadata.nlink() == ANONYMOUS_FILE_LINK_COUNT,
        "anonymous retained clone was linked early"
    );
    anyhow::ensure!(
        metadata.mode() & PERMISSION_MODE_MASK == PRIVATE_FILE_MODE,
        "anonymous retained clone permissions are unsafe"
    );
    anyhow::ensure!(
        metadata.len() == length && length <= max_bytes,
        "anonymous retained clone length changed"
    );
    crate::private_acl::require_empty(file, path)
}

#[cfg(target_os = "linux")]
fn verify_linked_clone(
    file: &File,
    parent: RawFd,
    name: &CStr,
    path: &Path,
    max_bytes: u64,
) -> Result<()> {
    let descriptor = FileIdentity::inspect_file_bounded(file, path, max_bytes)?;
    let named = open_named(parent, name, path, read_flags(), max_bytes)?;
    anyhow::ensure!(
        named.identity.unchanged(descriptor),
        "published retained clone changed"
    );
    Ok(())
}

#[cfg(any(test, feature = "persistence-test-support"))]
type FallibleHook = Option<Box<dyn FnOnce() -> Result<()>>>;

#[cfg(any(test, feature = "persistence-test-support"))]
thread_local! {
    static RETAINED_CLONE_HOOK: std::cell::RefCell<FallibleHook> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn set_retained_clone_hook<F: FnOnce() -> Result<()> + 'static>(hook: F) {
    RETAINED_CLONE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

fn run_clone_test_hook() -> Result<()> {
    #[cfg(any(test, feature = "persistence-test-support"))]
    return RETAINED_CLONE_HOOK.with(|slot| match slot.borrow_mut().take() {
        Some(hook) => hook(),
        None => Ok(()),
    });
    #[cfg(not(any(test, feature = "persistence-test-support")))]
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/private/private_file_clone_tests.rs"]
mod tests;
