use super::cleanup_issues::CleanupIssues;
use super::harness_namespace_retirement::NamespaceRetirement;
use super::*;

const CLEANUP_FAILURE_DETAIL: &str = "runtime harness cleanup failed";
const TEST_RETIREMENT_FAILURE_DETAIL: &str = "test Harness namespace retirement failed";
const SOCKET_NAMESPACE_REMAINED: &str =
    "test Harness socket namespace remained after confirmed process cleanup";

impl Drop for Harness {
    fn drop(&mut self) {
        let paths = self.paths();
        let trees = std::mem::take(
            self.cleanup_trees
                .get_mut()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        let (mut cleanup, _) = {
            let tracked = self
                .runtimes
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            RuntimeCleanup::with_trees(&paths, &tracked, trees)
        };
        let mut issues = CleanupIssues::default();
        let retirement = if let Some(retirement) = self.namespace_retirement.take() {
            // Explicit namespace-loss scenarios keep their original adversarial ordering.
            cleanup.request();
            issues.record(self.host.stop_result());
            cleanup.force();
            issues.record(cleanup.finish());
            Some(retirement)
        } else {
            // Stop supervision first, then settle exact runtimes while their durable
            // registration home still exists. Only then may the fixture retire its home.
            let host_stopped = issues.record(self.host.stop_result()).is_some();
            cleanup.request();
            cleanup.force();
            let runtimes_stopped = issues.record(cleanup.finish()).is_some();
            if host_stopped && runtimes_stopped
                && issues.record(settle_socket_namespace(&paths)).is_some()
            {
                Some(NamespaceRetirement::retire(&mut self.home))
            } else {
                self.home.disable_cleanup(true);
                None
            }
        };
        if let Some(retirement) = retirement {
            issues.record(retirement.finish());
        }
        report_cleanup_failure(issues.finish());
    }
}

fn settle_socket_namespace(paths: &Paths) -> anyhow::Result<()> {
    use yolo_runtime::paths::socket_namespace::NamespaceRelease;

    const OWNERS_REMAINED: &str = "test Harness socket namespace owners did not release custody before the process exit deadline";
    let deadline = Instant::now() + PROCESS_EXIT_TIMEOUT;
    loop {
        anyhow::ensure!(Instant::now() < deadline, OWNERS_REMAINED);
        let namespace = paths.retain_socket_namespace()?;
        let directory = namespace.directory()?;
        match namespace.release()? {
            NamespaceRelease::Removed => {
                return match std::fs::symlink_metadata(directory.path()) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(error) => Err(error.into()),
                    Ok(_) => anyhow::bail!(
                        "{SOCKET_NAMESPACE_REMAINED}: {}",
                        directory.path().display()
                    ),
                };
            }
            NamespaceRelease::Preserved => anyhow::bail!(
                "{SOCKET_NAMESPACE_REMAINED}: {}",
                directory.path().display()
            ),
            NamespaceRelease::Busy => {
                let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                    anyhow::bail!("{OWNERS_REMAINED}: {}", directory.path().display());
                };
                std::thread::sleep(remaining.min(PROCESS_POLL_INTERVAL));
            }
        }
    }
}

impl Harness {
    #[cfg(test)]
    pub(crate) fn retire_namespace_for_test(&mut self) -> PathBuf {
        let retirement = NamespaceRetirement::retire(&mut self.home);
        assert!(
            !self.home.path().exists(),
            "{TEST_RETIREMENT_FAILURE_DETAIL}"
        );
        let path = retirement.path_for_test().to_path_buf();
        self.namespace_retirement = Some(retirement);
        path
    }

    pub(crate) async fn restart_host_hard(&mut self) {
        self.kill_host_hard();
        self.replace_host(spawn_host(self.home.path())).await;
    }
}

fn report_cleanup_failure(result: anyhow::Result<()>) {
    let Err(error) = result else {
        return;
    };
    if std::thread::panicking() {
        tracing::error!(%error, "{CLEANUP_FAILURE_DETAIL}");
    } else {
        panic!("{CLEANUP_FAILURE_DETAIL}: {error:#}");
    }
}
