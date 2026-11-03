use crate::paths::{
    Paths,
    socket_namespace::{NamespaceRelease, SocketNamespace},
};
use anyhow::{Context, Result, ensure};
use std::path::Path;

const HOME_SCOPE_DETAIL: &str = "Host fixture home is outside its owned directory";
const NAMESPACE_RETAIN_DETAIL: &str = "retain Host fixture socket namespace";
const NAMESPACE_RELEASE_DETAIL: &str = "release Host fixture namespace before deleting its home";
const HOME_REMOVE_DETAIL: &str = "remove settled Host fixture directory";
const UNWIND_DETAIL: &str = "preserving Host fixture directory while unwinding";

// Declare this after Host, task, listener and custody owners in every fixture.
pub(in crate::host) struct HostTestDirectory {
    directory: Option<tempfile::TempDir>,
    namespace: Option<SocketNamespace>,
}

impl HostTestDirectory {
    pub(in crate::host) fn capture(mut directory: tempfile::TempDir, paths: &Paths) -> Self {
        directory.disable_cleanup(true);
        assert!(
            paths
                .home()
                .canonicalize()
                .unwrap()
                .starts_with(directory.path().canonicalize().unwrap()),
            "{HOME_SCOPE_DETAIL}"
        );
        let namespace = paths
            .retain_socket_namespace()
            .expect(NAMESPACE_RETAIN_DETAIL);
        Self {
            directory: Some(directory),
            namespace: Some(namespace),
        }
    }

    pub(in crate::host) fn path(&self) -> &Path {
        self.directory.as_ref().unwrap().path()
    }

    fn finish(directory: tempfile::TempDir, namespace: SocketNamespace) -> Result<()> {
        let release = namespace.release().context(NAMESPACE_RELEASE_DETAIL)?;
        ensure!(
            release == NamespaceRelease::Removed,
            "{NAMESPACE_RELEASE_DETAIL}: {release:?}"
        );
        directory.close().context(HOME_REMOVE_DETAIL)
    }
}

impl Drop for HostTestDirectory {
    fn drop(&mut self) {
        let mut directory = self.directory.take().unwrap();
        let mut namespace = self.namespace.take().unwrap();
        directory.disable_cleanup(true);
        if std::thread::panicking() {
            namespace.preserve_on_release();
            eprintln!("{UNWIND_DETAIL}: {}", directory.path().display());
            return;
        }
        let path = directory.path().to_path_buf();
        if let Err(error) = Self::finish(directory, namespace) {
            panic!("{NAMESPACE_RELEASE_DETAIL}: {}; {error:#}", path.display());
        }
    }
}

#[cfg(feature = "yolo-integration-tests")]
#[path = "../integration_tests/test_directory_lifecycle.rs"]
mod tests;
