use crate::snapshot::GateSnapshot;
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::path::{Component, Path};
use std::sync::Arc;
use yolo_release_bazel_output::BazelOutputBinding;
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::{BAZEL_BIN_LINK_NAME, PACKAGE_ARTIFACT_RELATIVE_PATH};
use yolo_release_filesystem::{DirectoryHandle, TreeManifest, set_read_only};

impl GateSnapshot {
    pub fn pin_artifact(&self) -> Result<PinnedArtifact> {
        let binding = BazelOutputBinding::capture(
            &self.output,
            &self.links,
            BAZEL_BIN_LINK_NAME,
            PACKAGE_ARTIFACT_RELATIVE_PATH,
        )?;
        let artifact = PinnedArtifact::from_opened(binding.file())?;
        binding.verify()?;
        Ok(artifact)
    }

    pub fn source(&self) -> &DirectoryHandle {
        &self.workspace
    }

    pub fn temporary(&self) -> &DirectoryHandle {
        &self.temporary
    }

    pub fn home(&self) -> &DirectoryHandle {
        &self.home
    }

    pub fn output(&self) -> &DirectoryHandle {
        &self.output
    }

    pub fn links(&self) -> &DirectoryHandle {
        &self.links
    }

    pub fn linux_artifacts(&self) -> &DirectoryHandle {
        &self.linux_artifacts
    }

    pub fn seal_linux_artifacts(&mut self) -> Result<()> {
        set_read_only(self.linux_artifacts.display_path())?;
        self.linux_artifact_manifest = Some(TreeManifest::capture(&self.linux_artifacts)?);
        self.verify_integrity()
    }

    pub fn open_source_file(&self, relative: &str) -> Result<Arc<File>> {
        let components = Path::new(relative)
            .components()
            .map(|component| match component {
                Component::Normal(name) => name
                    .to_str()
                    .context("fixed source component must be UTF-8"),
                _ => anyhow::bail!("fixed source path must be relative and normalized"),
            })
            .collect::<Result<Vec<_>>>()?;
        let (name, parents) = components
            .split_last()
            .context("fixed source path must not be empty")?;
        let mut directory = self.workspace.clone();
        for component in parents {
            directory = DirectoryHandle::open_at(&directory, component)?;
        }
        let name = CString::new(*name)?;
        let descriptor = unsafe {
            libc::openat(
                directory.raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("open fixed source file {relative}"));
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        ensure!(
            file.metadata()?.is_file(),
            "fixed source input must be a file"
        );
        Ok(Arc::new(file))
    }

    pub fn open_linux_manifest(&self) -> Result<Arc<File>> {
        let name = CString::new(yolo_release_contracts::LINUX_RUNTIME_MANIFEST_NAME)?;
        let descriptor = unsafe {
            libc::openat(
                self.linux_artifacts.raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error()).context("open Linux runtime manifest");
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        ensure!(
            file.metadata()?.is_file(),
            "Linux runtime manifest must be a file"
        );
        Ok(Arc::new(file))
    }
}

#[cfg(any(test, feature = "yolo-test-support"))]
mod test_support {
    use super::*;
    use anyhow::Result;
    use std::fs;

    impl GateSnapshot {
        pub fn fixture() -> Result<Self> {
            Self::fixture_with_source(&[])
        }

        pub fn fixture_with_source(entries: &[(&str, &[u8])]) -> Result<Self> {
            let mut snapshot = Self::empty()?;
            for (relative, bytes) in entries {
                let path = snapshot.workspace.display_path().join(relative);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, bytes)?;
            }
            seal(&mut snapshot)?;
            Ok(snapshot)
        }
    }

    fn seal(snapshot: &mut GateSnapshot) -> Result<()> {
        set_read_only(snapshot.workspace.display_path())?;
        snapshot.source_manifest = Some(TreeManifest::capture(&snapshot.workspace)?);
        Ok(())
    }
}
