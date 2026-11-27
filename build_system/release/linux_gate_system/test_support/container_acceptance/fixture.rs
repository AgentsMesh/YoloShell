use super::constants::*;
use anyhow::Result;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use yolo_release_contracts::*;
use yolo_release_filesystem::{DirectoryHandle, DirectorySeal, TemporaryDirectory};

pub struct FixtureTool {
    root: TemporaryDirectory,
    seal: DirectorySeal,
    executable: PathBuf,
}

impl FixtureTool {
    pub fn create() -> Result<Self> {
        let mut root = TemporaryDirectory::create(TOOL_PREFIX)?;
        let executable = root.path().join(TOOL_NAME);
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(PRIVATE_FILE_MODE)
            .open(&executable)?;
        write!(
            file,
            "#!{CONTAINER_BASH}\n\
             set -eu\n\
             if test \"$1\" = '{BAZEL_VERSION_ARGUMENT}'; then\n\
               printf '%s\\n' '{LINUX_BAZEL_VERSION}'\n\
               exit 0\n\
             fi\n\
             trap '' TERM HUP INT\n\
             printf '%s\\n' '{READY_FILE}' > '{DOCKER_ARTIFACT_DIRECTORY}/{READY_FILE}'\n\
             deadline=$((SECONDS + {}))\n\
             while ((SECONDS < deadline)); do /bin/sleep {}; done\n\
             exit {FIXTURE_EXIT}\n",
            FIXTURE_LIFETIME.as_secs(),
            FIXTURE_POLL.as_secs(),
        )?;
        file.sync_all()?;
        drop(file);
        fs::set_permissions(
            &executable,
            fs::Permissions::from_mode(READ_ONLY_EXECUTABLE_MODE),
        )?;
        let seal = root.seal()?;
        Ok(Self {
            root,
            seal,
            executable,
        })
    }

    pub fn path(&self) -> &Path {
        &self.executable
    }

    pub fn root_path(&self) -> PathBuf {
        self.root.path()
    }

    pub fn directory(&self) -> &DirectoryHandle {
        self.root.handle()
    }

    pub fn verify(&self) -> Result<()> {
        self.root.handle().verify_seal(&self.seal)
    }

    pub fn close(mut self) -> Result<()> {
        self.verify()?;
        self.root.cleanup()
    }
}
