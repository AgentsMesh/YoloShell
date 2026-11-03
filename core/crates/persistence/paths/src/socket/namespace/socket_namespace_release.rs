use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamespaceRelease {
    Busy,
    Preserved,
    Removed,
}

impl SocketNamespace {
    /// Release this lease and report cleanup failures to callers that must settle
    /// transport state before retiring its durable home. Peers still retain custody.
    pub fn release(mut self) -> Result<NamespaceRelease> {
        if self.preserve_directory_on_release {
            return Ok(NamespaceRelease::Preserved);
        }
        self.preserve_directory_on_release = true;
        self.release_inner()
    }

    fn release_inner(&mut self) -> Result<NamespaceRelease> {
        registered_cleanup::cleanup(self)
    }
}

impl Drop for SocketNamespace {
    fn drop(&mut self) {
        if self.preserve_directory_on_release {
            return;
        }
        if let Err(error) = self.release_inner() {
            tracing::warn!(
                %error,
                namespace = %self.location.directory.display(),
                message = SOCKET_NAMESPACE_CLEANUP_FAILED_LOG
            );
        }
    }
}
