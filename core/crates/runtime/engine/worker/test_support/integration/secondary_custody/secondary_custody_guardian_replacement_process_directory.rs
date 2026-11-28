use crate::paths::{
    Paths,
    socket_namespace::{NamespaceRelease, SocketNamespace},
};

const NAMESPACE_RELEASE_DETAIL: &str = "release replacement fixture namespace before its home";

// Fixture declares this last so process and WorkerNamespace owners finish first.
pub(super) struct FixtureDirectory {
    directory: tempfile::TempDir,
    namespace: Option<SocketNamespace>,
}

impl FixtureDirectory {
    pub(super) fn new(directory: tempfile::TempDir, paths: &Paths) -> Self {
        let namespace = paths.retain_socket_namespace().unwrap();
        Self {
            directory,
            namespace: Some(namespace),
        }
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        self.directory.disable_cleanup(true);
        let mut namespace = self.namespace.take().unwrap();
        if std::thread::panicking() {
            namespace.preserve_on_release();
            return;
        }
        assert_eq!(
            namespace.release().expect(NAMESPACE_RELEASE_DETAIL),
            NamespaceRelease::Removed,
            "{NAMESPACE_RELEASE_DETAIL}"
        );
        self.directory.disable_cleanup(false);
    }
}
