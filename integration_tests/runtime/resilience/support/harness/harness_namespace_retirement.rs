use super::cleanup_issues::CleanupIssues;
use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use uuid::Uuid;

const TOMBSTONE_NAME_SEPARATOR: &str = ".yolo-retired-";
const RENAME_CONTEXT: &str = "atomically retire runtime harness namespace";
const REMOVE_CONTEXT: &str = "remove retired runtime harness namespace";
#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
const REMOVE_SETTLEMENT_TIMEOUT: Duration = yolo_resilience_support::
    secondary_custody_adversarial_observation::GUARDIAN_NAMESPACE_LOSS_SETTLEMENT_TIMEOUT;
#[cfg(not(all(target_os = "macos", feature = "adversarial-test-hooks")))]
const REMOVE_SETTLEMENT_TIMEOUT: Duration = super::PROCESS_EXIT_TIMEOUT;

pub(super) struct NamespaceRetirement {
    cleanup_path: PathBuf,
    rename_error: Option<anyhow::Error>,
}

impl NamespaceRetirement {
    pub(super) fn retire(home: &mut TempDir) -> Self {
        Self::retire_with(home, &Filesystem)
    }

    fn retire_with(home: &mut TempDir, io: &impl NamespaceRetirementIo) -> Self {
        let original = home.path().to_path_buf();
        let tombstone = tombstone_path(&original);
        home.disable_cleanup(true);
        match io.rename(&original, &tombstone).with_context(|| {
            format!(
                "{RENAME_CONTEXT}: {} -> {}",
                original.display(),
                tombstone.display()
            )
        }) {
            Ok(()) => Self {
                cleanup_path: tombstone,
                rename_error: None,
            },
            Err(error) => Self {
                cleanup_path: original,
                rename_error: Some(error),
            },
        }
    }

    pub(super) fn finish(self) -> Result<()> {
        self.finish_with(&Filesystem)
    }

    fn finish_with(self, io: &impl NamespaceRetirementIo) -> Result<()> {
        let mut issues = CleanupIssues::default();
        issues.record_error(self.rename_error);
        issues.record(remove_with_retry(io, &self.cleanup_path));
        issues.finish()
    }

    #[cfg(test)]
    pub(super) fn path_for_test(&self) -> &Path {
        &self.cleanup_path
    }
}

fn tombstone_path(original: &Path) -> PathBuf {
    let mut name = original
        .file_name()
        .map_or_else(OsString::new, OsString::from);
    name.push(TOMBSTONE_NAME_SEPARATOR);
    name.push(Uuid::new_v4().to_string());
    original.with_file_name(name)
}

fn remove_with_retry(io: &impl NamespaceRetirementIo, path: &Path) -> Result<()> {
    let deadline = io.now() + REMOVE_SETTLEMENT_TIMEOUT;
    loop {
        match io.remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                let now = io.now();
                if now >= deadline {
                    return Err(error)
                        .with_context(|| format!("{REMOVE_CONTEXT}: {}", path.display()));
                }
                io.pause(super::PROCESS_POLL_INTERVAL.min(deadline.duration_since(now)));
            }
        }
    }
}

trait NamespaceRetirementIo {
    fn rename(&self, source: &Path, target: &Path) -> std::io::Result<()>;
    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()>;
    fn pause(&self, duration: Duration);
    fn now(&self) -> Instant;
}

struct Filesystem;

impl NamespaceRetirementIo for Filesystem {
    fn rename(&self, source: &Path, target: &Path) -> std::io::Result<()> {
        std::fs::rename(source, target)
    }

    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    fn pause(&self, duration: Duration) {
        std::thread::sleep(duration);
    }

    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
#[path = "harness_namespace_retirement_tests.rs"]
mod tests;
