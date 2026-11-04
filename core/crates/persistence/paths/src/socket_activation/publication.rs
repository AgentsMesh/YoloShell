use super::*;

pub struct SocketPublication {
    pub(super) directory: PrivateDirectory,
    pub(super) name: CString,
    pub(super) identity: SocketIdentity,
    #[cfg(any(test, feature = "persistence-test-support"))]
    pub(super) before_unlink: Option<Box<dyn FnOnce() + Send + Sync>>,
}

impl SocketPublication {
    pub(super) fn new(
        directory: PrivateDirectory,
        name: CString,
        identity: SocketIdentity,
    ) -> Self {
        Self {
            directory,
            name,
            identity,
            #[cfg(any(test, feature = "persistence-test-support"))]
            before_unlink: None,
        }
    }

    pub(super) fn verify_published(&self) -> Result<()> {
        self.directory.sync()?;
        inspect_at(&self.directory, &self.name, true)?.context(PUBLISH_DISAPPEARED_ERROR)?;
        anyhow::ensure!(publication_is_owned_result(self)?, PUBLISH_CHANGED_ERROR);
        Ok(())
    }

    pub fn withdraw(mut self) -> Result<()> {
        anyhow::ensure!(publication_is_owned_result(&self)?, PUBLISH_CHANGED_ERROR);
        self.run_before_unlink();
        anyhow::ensure!(publication_is_owned_result(&self)?, PUBLISH_CHANGED_ERROR);
        unlink_at(&self.directory, &self.name)?;
        self.directory.sync()?;
        anyhow::ensure!(
            inspect_at(&self.directory, &self.name, false)?.is_none(),
            TARGET_REMOVE_ERROR
        );
        Ok(())
    }

    #[cfg(any(test, feature = "persistence-test-support"))]
    pub(crate) fn set_before_unlink<F>(&mut self, hook: F)
    where
        F: FnOnce() + Send + Sync + 'static,
    {
        assert!(self.before_unlink.is_none(), "{CLEANUP_HOOK_ALREADY_SET}");
        self.before_unlink = Some(Box::new(hook));
    }

    #[cfg(any(test, feature = "persistence-test-support"))]
    fn run_before_unlink(&mut self) {
        if let Some(hook) = self.before_unlink.take() {
            hook();
        }
    }

    #[cfg(not(any(test, feature = "persistence-test-support")))]
    fn run_before_unlink(&mut self) {}
}

impl Drop for SocketPublication {
    fn drop(&mut self) {
        if publication_is_owned(self) {
            self.run_before_unlink();
            if publication_is_owned(self) {
                let _ = unlink_at(&self.directory, &self.name);
                let _ = self.directory.sync();
            }
        }
    }
}

fn publication_is_owned(publication: &SocketPublication) -> bool {
    publication_is_owned_result(publication).unwrap_or(false)
}

fn publication_is_owned_result(publication: &SocketPublication) -> Result<bool> {
    socket_matches_at(
        &publication.directory,
        &publication.name,
        true,
        publication.identity,
        crate::paths::constants::SOCKET_ENDPOINT_LINK_COUNT,
    )
}
