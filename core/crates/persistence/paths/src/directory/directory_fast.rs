use super::PrivateDirectory;
#[cfg(target_os = "macos")]
use anyhow::Context;
use anyhow::Result;

impl PrivateDirectory {
    #[cfg(target_os = "macos")]
    pub fn verify_descriptor_path(&self) -> Result<()> {
        use std::ffi::{CStr, OsStr};
        use std::os::fd::AsRawFd;
        use std::os::unix::ffi::OsStrExt;
        use std::path::Path;

        crate::private_acl::require_empty(&self.file, &self.path)?;
        anyhow::ensure!(
            super::DirectoryIdentity::inspect_private(&self.file.metadata()?, &self.path)?
                == self.identity,
            "private directory descriptor changed: {}",
            self.path.display()
        );
        let mut buffer = [0_i8; libc::PATH_MAX as usize];
        let result =
            unsafe { libc::fcntl(self.file.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) };
        if result != 0 {
            return Err(std::io::Error::last_os_error()).context("resolve private directory path");
        }
        let bytes = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_bytes();
        let actual = Path::new(OsStr::from_bytes(bytes));
        let expected = super::super::platform::platform_path(&self.path);
        anyhow::ensure!(actual == expected, "private directory path changed");
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn verify_descriptor_path(&self) -> Result<()> {
        self.verify_path()
    }
}
