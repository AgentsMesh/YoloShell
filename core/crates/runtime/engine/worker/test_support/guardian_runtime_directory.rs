const GUARDIAN_HOME_CLEANUP_DETAIL: &str = "retire test guardians before deleting runtime home";
const GUARDIAN_HOME_UNSETTLED_DETAIL: &str =
    "runtime fixture creators were not joined; preserving their durable home";
const GUARDIAN_HOME_REMOVE_DETAIL: &str = "remove settled runtime fixture home";
const GUARDIAN_HOME_IDENTITY_DETAIL: &str = "runtime fixture cleanup paths belong to another home";
const GUARDIAN_NAMESPACE_RETAIN_DETAIL: &str =
    "retain runtime fixture namespace before guardian cleanup";
const GUARDIAN_NAMESPACE_RELEASE_DETAIL: &str =
    "release runtime fixture namespace before deleting its home";
#[path = "guardian_runtime_directory_diagnostics.rs"]
mod guardian_directory_diagnostics;

struct GuardianRuntimeDirectory {
    directory: Option<tempfile::TempDir>,
    cleanup_attempted: bool,
}

impl GuardianRuntimeDirectory {
    fn new() -> Self {
        Self {
            directory: Some(tempfile::tempdir().unwrap()),
            cleanup_attempted: false,
        }
    }

    fn path(&self) -> &Path {
        self.directory.as_ref().unwrap().path()
    }

    // Call only after every task or thread that can create guardians has joined.
    // An abort request alone does not establish that boundary.
    fn close(mut self, paths: &Paths) -> Result<()> {
        self.cleanup_attempted = true;
        if let Err(error) = self.settle(paths) {
            self.directory.as_mut().unwrap().disable_cleanup(true);
            return Err(error);
        }
        self.directory
            .take()
            .unwrap()
            .close()
            .context(GUARDIAN_HOME_REMOVE_DETAIL)
    }

    fn settle(&self, paths: &Paths) -> Result<()> {
        anyhow::ensure!(
            paths.home().canonicalize()? == self.path().canonicalize()?,
            GUARDIAN_HOME_IDENTITY_DETAIL
        );
        let mut namespace = paths
            .retain_socket_namespace()
            .context(GUARDIAN_NAMESPACE_RETAIN_DETAIL)?;
        let namespace_path = namespace.directory()?.path().to_path_buf();
        if let Err(error) = crate::custody_guardian::terminate_test_processes_for_home(self.path())
        {
            namespace.preserve_on_release();
            return Err(error).context(GUARDIAN_HOME_CLEANUP_DETAIL);
        }
        let release = namespace
            .release()
            .context(GUARDIAN_NAMESPACE_RELEASE_DETAIL)?;
        anyhow::ensure!(
            release == crate::paths::socket_namespace::NamespaceRelease::Removed,
            "{GUARDIAN_NAMESPACE_RELEASE_DETAIL}: {release:?}; {}",
            guardian_directory_diagnostics::capture(&namespace_path)
        );
        Ok(())
    }
}

impl Drop for GuardianRuntimeDirectory {
    fn drop(&mut self) {
        let Some(directory) = self.directory.as_mut() else {
            return;
        };
        // Creators may still run on an unwinding path. Preserve their namespace even
        // if the current registry is empty or its registered guardians were reclaimed.
        directory.disable_cleanup(true);
        if self.cleanup_attempted {
            return;
        }
        self.cleanup_attempted = true;
        let cleanup = crate::custody_guardian::terminate_test_processes_for_home(directory.path());
        let detail = format!(
            "{GUARDIAN_HOME_UNSETTLED_DETAIL}: {}; cleanup={cleanup:?}",
            directory.path().display()
        );
        if std::thread::panicking() {
            eprintln!("{detail}");
        } else {
            panic!("{detail}");
        }
    }
}
