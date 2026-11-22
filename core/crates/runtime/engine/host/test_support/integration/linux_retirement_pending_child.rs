use super::*;
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

const FREEZE_CONTROL: &std::ffi::CStr = c"cgroup.freeze";
const THAW_VALUE: &[u8] = b"0\n";
const FDINFO_DIRECTORY: &str = "/proc/self/fdinfo";
const MOUNT_ID_PREFIX: &str = "mnt_id:";
const CHANGED: &str = "pending retirement cleanup leaf identity changed";

pub(super) struct PendingChild {
    thaw: std::fs::File,
    child: Option<ArchiveTestProcess>,
}

impl PendingChild {
    pub(super) fn retain(fixture: &Fixture, child: ArchiveTestProcess) -> Self {
        let record: process::linux_containment::LinuxContainmentRecord = fixture
            .host
            .namespace
            .read_json(&fixture.containment_path())
            .unwrap();
        let leaf = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(record.delegated_root.join(&record.leaf_name))
            .unwrap();
        assert_eq!(
            leaf.metadata().unwrap().ino(),
            record.leaf_identity.inode,
            "{CHANGED}"
        );
        let fdinfo =
            std::fs::read_to_string(Path::new(FDINFO_DIRECTORY).join(leaf.as_raw_fd().to_string()))
                .unwrap();
        let mount = fdinfo
            .lines()
            .find_map(|line| line.strip_prefix(MOUNT_ID_PREFIX))
            .unwrap()
            .trim()
            .parse::<u64>()
            .unwrap();
        assert_eq!(mount, record.leaf_identity.mount_id, "{CHANGED}");
        let descriptor = unsafe {
            libc::openat(
                leaf.as_raw_fd(),
                FREEZE_CONTROL.as_ptr(),
                libc::O_WRONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        assert!(descriptor >= libc::c_int::default());
        let thaw = unsafe { std::fs::File::from_raw_fd(descriptor) };
        Self {
            thaw,
            child: Some(child),
        }
    }

    pub(super) fn finish(&mut self) -> std::io::Result<()> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        let result = self.thaw.write_all(THAW_VALUE);
        child.stop();
        result
    }
}

impl Drop for PendingChild {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            if std::thread::panicking() {
                eprintln!("pending retirement cleanup: {error}");
            } else {
                panic!("pending retirement cleanup: {error}");
            }
        }
    }
}
