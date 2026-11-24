use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt, PermissionsExt};
use std::path::Path;

const LOCAL_RACE_FILE_OFFSET: u64 = 0;
const SYSCALL_SUCCESS: libc::c_int = 0;
const PRIVATE_IDENTITY_MODE: u32 = 0o600;
const PUBLIC_KNOWN_HOSTS_MODE: u32 = 0o644;
const IDENTITY_RACE_FILE: &str = "identity-race";
const KNOWN_HOSTS_RACE_FILE: &str = "known-hosts-race";
const ORIGINAL_IDENTITY: &[u8] = b"trusted-identity";
const CHANGED_IDENTITY: &[u8] = b"changed-identity";
const ORIGINAL_KNOWN_HOST: &[u8] = b"h ssh-ed25519 key\n";
const CHANGED_KNOWN_HOST: &[u8] = b"x ssh-ed25519 key\n";
const MTIME_RESTORE_ERROR: &str = "test mutation did not restore mtime";

#[test]
fn identity_rejects_equal_length_rewrite_with_restored_bytes_and_mtime() {
    assert_eq!(ORIGINAL_IDENTITY.len(), CHANGED_IDENTITY.len());
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(IDENTITY_RACE_FILE);
    write_mode(&path, ORIGINAL_IDENTITY, PRIVATE_IDENTITY_MODE);

    let result = super::super::local_file::read_identity_with_test_hook(&path, |opened| {
        tamper_and_restore(opened, ORIGINAL_IDENTITY, CHANGED_IDENTITY)
    });

    assert!(result.is_err());
    assert_eq!(fs::read(path).unwrap(), ORIGINAL_IDENTITY);
}

#[test]
fn known_hosts_rejects_equal_length_rewrite_with_restored_bytes_and_mtime() {
    assert_eq!(ORIGINAL_KNOWN_HOST.len(), CHANGED_KNOWN_HOST.len());
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(KNOWN_HOSTS_RACE_FILE);
    write_mode(&path, ORIGINAL_KNOWN_HOST, PUBLIC_KNOWN_HOSTS_MODE);

    let result = super::super::local_file::read_known_hosts_with_test_hook(&path, |opened| {
        tamper_and_restore(opened, ORIGINAL_KNOWN_HOST, CHANGED_KNOWN_HOST)
    });

    assert!(result.is_err());
    assert_eq!(fs::read(path).unwrap(), ORIGINAL_KNOWN_HOST);
}

fn write_mode(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn tamper_and_restore(path: &Path, original: &[u8], changed: &[u8]) -> std::io::Result<()> {
    let metadata = fs::metadata(path)?;
    let file = OpenOptions::new().write(true).open(path)?;
    file.write_all_at(changed, LOCAL_RACE_FILE_OFFSET)?;
    file.sync_data()?;
    file.write_all_at(original, LOCAL_RACE_FILE_OFFSET)?;
    file.sync_data()?;
    let times = [
        libc::timespec {
            tv_sec: metadata.atime(),
            tv_nsec: metadata.atime_nsec(),
        },
        libc::timespec {
            tv_sec: metadata.mtime(),
            tv_nsec: metadata.mtime_nsec(),
        },
    ];
    if unsafe { libc::futimens(file.as_raw_fd(), times.as_ptr()) } != SYSCALL_SUCCESS {
        return Err(std::io::Error::last_os_error());
    }
    let restored = fs::metadata(path)?;
    if metadata.mtime() == restored.mtime() && metadata.mtime_nsec() == restored.mtime_nsec() {
        Ok(())
    } else {
        Err(std::io::Error::other(MTIME_RESTORE_ERROR))
    }
}
