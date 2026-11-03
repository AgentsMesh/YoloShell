use super::*;
use std::ffi::CString;
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};

const FIRST_DIRECTORY: &str = "first";
const SECOND_DIRECTORY: &str = "second";
const UNKNOWN_GENERATION_FILE: &str = "unknown";
const MISSING_TARGET: &str = "missing-target";
const DEVICE_ENTRY: &str = "/dev/null";
const INVALID_DEVICE: libc::dev_t = -1;
const INODE_INCREMENT: u64 = 1;
const CLEAR_FLAGS: libc::c_uint = 0;
const SYSTEM_CALL_SUCCESS: libc::c_int = 0;
const INITIAL_PENDING_INDEX: usize = 0;
const PENDING_RETRY_INDEX: usize = 1;
const FILE_BYTES: &[u8] = b"pending";
const BEFORE_OPEN_FAILURE: &str = "injected before-open failure";

#[test]
fn generation_identity_rejects_inode_device_and_metadata_replacements() {
    let root = tempfile::tempdir().unwrap();
    let first_path = root.path().join(FIRST_DIRECTORY);
    let second_path = root.path().join(SECOND_DIRECTORY);
    create_private_directory(&first_path);
    create_private_directory(&second_path);
    let first = fs::metadata(&first_path).unwrap();
    let second = fs::metadata(&second_path).unwrap();
    let identity = io::GenerationIdentity::inspect(&first, PRIVATE_DIRECTORY_MODE).unwrap();

    assert!(!identity.matches(&second));
    assert!(!identity.matches(&fs::metadata(DEVICE_ENTRY).unwrap()));
    let mut metadata = unsafe { std::mem::zeroed::<libc::stat>() };
    metadata.st_dev = first.dev().try_into().unwrap();
    metadata.st_ino = first.ino().wrapping_add(INODE_INCREMENT);
    assert!(!identity.matches_stat(&metadata));
    metadata.st_dev = INVALID_DEVICE;
    metadata.st_ino = first.ino();
    assert!(!identity.matches_stat(&metadata));
}

#[test]
fn immutable_directory_rejects_mode_repair() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(FIRST_DIRECTORY);
    create_private_directory(&path);
    fs::set_permissions(&path, fs::Permissions::from_mode(IMMUTABLE_DIRECTORY_MODE)).unwrap();
    let directory = File::open(&path).unwrap();
    let immutable = UserImmutable::new(&path);

    assert!(io::set_directory_mode(&directory, &path, PRIVATE_DIRECTORY_MODE).is_err());
    drop(immutable);
}

#[test]
fn known_generation_symlink_is_rejected_before_acl_inspection() {
    let root = tempfile::tempdir().unwrap();
    let directory = File::open(root.path()).unwrap();
    symlink(MISSING_TARGET, root.path().join(GENERATION_CONFIG_FILE)).unwrap();

    assert!(entries::ensure_known(&directory, root.path()).is_err());
}

#[test]
fn cleanup_failure_preserves_an_immutable_pending_generation() {
    let base = tempfile::tempdir().unwrap();
    let root_path = base.path().join(CONFIG_GENERATIONS_DIRECTORY);
    let root = GenerationRoot::ensure(&root_path).unwrap();
    let pending = PendingDirectory::create(&root).unwrap();
    fs::set_permissions(
        &pending.path,
        fs::Permissions::from_mode(IMMUTABLE_DIRECTORY_MODE),
    )
    .unwrap();
    let immutable = UserImmutable::new(&pending.path);
    let path = pending.path.clone();

    drop(pending);

    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & PERMISSION_BITS_MASK,
        IMMUTABLE_DIRECTORY_MODE
    );
    drop(immutable);
}

#[test]
fn descriptor_rebinding_disarms_pending_cleanup() {
    let base = tempfile::tempdir().unwrap();
    let root_path = base.path().join(CONFIG_GENERATIONS_DIRECTORY);
    let root = GenerationRoot::ensure(&root_path).unwrap();
    let pending = PendingDirectory::create(&root).unwrap();
    let replacement_path = base.path().join(SECOND_DIRECTORY);
    create_private_directory(&replacement_path);
    let replacement = File::open(&replacement_path).unwrap();
    let pending_descriptor = pending.directory.as_raw_fd();
    assert_eq!(
        unsafe { libc::dup2(replacement.as_raw_fd(), pending_descriptor) },
        pending_descriptor
    );
    let pending_path = pending.path.clone();

    assert!(
        pending
            .verify_source(&root, PRIVATE_DIRECTORY_MODE)
            .is_err()
    );

    drop(pending);

    assert!(pending_path.exists());
}

#[test]
fn transient_slot_removal_retries_the_next_staging_index() {
    let base = tempfile::tempdir().unwrap();
    let root_path = base.path().join(CONFIG_GENERATIONS_DIRECTORY);
    let root = GenerationRoot::ensure(&root_path).unwrap();
    let first_entry = format!("{PENDING_GENERATION_PREFIX}{INITIAL_PENDING_INDEX}");
    let first_path = root_path.join(&first_entry);
    let retry_path = root_path.join(format!(
        "{PENDING_GENERATION_PREFIX}{PENDING_RETRY_INDEX}{PENDING_GENERATION_PAYLOAD_SUFFIX}"
    ));
    let pending = PendingDirectory::create_using(&root, |name| {
        if name.to_bytes() == first_entry.as_bytes() {
            fs::remove_dir(&first_path)?;
        }
        Ok(())
    })
    .unwrap();

    assert!(!first_path.exists());
    assert_eq!(pending.path, retry_path);
}

#[test]
fn before_open_failure_leaves_a_recyclable_staging_slot() {
    let base = tempfile::tempdir().unwrap();
    let root_path = base.path().join(CONFIG_GENERATIONS_DIRECTORY);
    let root = GenerationRoot::ensure(&root_path).unwrap();

    let error = PendingDirectory::create_using(&root, |_| anyhow::bail!(BEFORE_OPEN_FAILURE))
        .err()
        .unwrap();

    assert!(error.to_string().contains(BEFORE_OPEN_FAILURE));
    drop(PendingDirectory::create(&root).unwrap());
}

#[test]
fn pending_writes_reject_unknown_names_and_sealing_rejects_unknown_entries() {
    let base = tempfile::tempdir().unwrap();
    let root_path = base.path().join(CONFIG_GENERATIONS_DIRECTORY);
    let root = GenerationRoot::ensure(&root_path).unwrap();
    let pending = PendingDirectory::create(&root).unwrap();
    let unknown = entry_name(UNKNOWN_GENERATION_FILE).unwrap();

    assert!(pending.write_file(&unknown, FILE_BYTES).is_err());
    fs::write(pending.path.join(UNKNOWN_GENERATION_FILE), FILE_BYTES).unwrap();
    assert!(pending.seal(&root).is_err());
}

fn create_private_directory(path: &Path) {
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE)).unwrap();
}

struct UserImmutable(PathBuf);

impl UserImmutable {
    fn new(path: &Path) -> Self {
        set_flags(path, libc::UF_IMMUTABLE);
        Self(path.to_owned())
    }
}

impl Drop for UserImmutable {
    fn drop(&mut self) {
        set_flags(&self.0, CLEAR_FLAGS);
    }
}

fn set_flags(path: &Path, flags: libc::c_uint) {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::chflags(path.as_ptr(), flags) },
        SYSTEM_CALL_SUCCESS
    );
}
