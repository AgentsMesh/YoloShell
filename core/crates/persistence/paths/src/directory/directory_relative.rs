use super::*;
use crate::paths::identity::ObjectIdentity;

impl PrivateDirectory {
    pub fn durable_identity(&self) -> Result<crate::paths::identity::DurableObjectIdentity> {
        use std::os::fd::AsFd;
        self.verify_path()?;
        let identity = crate::paths::identity::DurableObjectIdentity::capture(self.file.as_fd())?;
        self.verify_path()?;
        Ok(identity)
    }

    pub fn object_identity(&self) -> Result<ObjectIdentity> {
        ObjectIdentity::from_file(&self.file)?
            .context("private directory birth identity is unavailable")
    }

    pub fn same_identity(&self, other: &Self) -> bool {
        self.identity == other.identity
    }

    pub fn from_opened(file: File, path: &Path) -> Result<Self> {
        crate::private_acl::require_empty(&file, path)?;
        let identity = DirectoryIdentity::inspect_private(&file.metadata()?, path)?;
        Ok(Self {
            file,
            identity,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            file: self.file.try_clone().context("clone private directory")?,
            identity: self.identity,
            path: self.path.clone(),
        })
    }

    pub fn open_descendant(&self, path: &Path) -> Result<Self> {
        self.descendant(path, false)
    }

    pub fn ensure_descendant(&self, path: &Path) -> Result<Self> {
        self.descendant(path, true)
    }

    pub(crate) fn ensure_descendant_with_commit_fence<F>(
        &self,
        path: &Path,
        mut commit_fence: F,
    ) -> Result<Self>
    where
        F: FnMut() -> Result<()>,
    {
        self.descendant_with(path, true, &mut commit_fence)
    }

    fn descendant(&self, path: &Path, ensure: bool) -> Result<Self> {
        self.descendant_with(path, ensure, &mut || Ok(()))
    }

    fn descendant_with<F>(&self, path: &Path, ensure: bool, commit_fence: &mut F) -> Result<Self>
    where
        F: FnMut() -> Result<()>,
    {
        super::super::validation::validate_absolute(path)?;
        let relative = path.strip_prefix(&self.path).with_context(|| {
            format!(
                "private directory {} is outside anchor {}",
                path.display(),
                self.path.display()
            )
        })?;
        let mut current = self.file.try_clone().context("clone private anchor")?;
        let mut current_path = self.path.clone();
        for component in relative.components() {
            let Component::Normal(name) = component else {
                anyhow::bail!("invalid private descendant: {}", path.display());
            };
            let name_c = entry_c_string(name)?;
            let (next, created) = if ensure {
                open_or_create_with_fence(&current, &name_c, &current_path, &mut *commit_fence)?
            } else {
                (open_directory(current.as_raw_fd(), &name_c)?, false)
            };
            current_path.push(name);
            if created {
                set_private_mode(&next, &current_path)?;
            }
            crate::private_acl::require_empty(&next, &current_path)?;
            DirectoryIdentity::inspect_private(&next.metadata()?, &current_path)?;
            current = next;
        }
        Self::from_opened(current, path)
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/durable/durable_directory_identity.rs"]
mod durable_identity_tests;
