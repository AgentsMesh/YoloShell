use super::*;
use std::cell::Cell;

enum RemoveBehavior {
    Real,
    Fail(std::io::ErrorKind),
    Transient {
        remaining: Cell<usize>,
        kind: std::io::ErrorKind,
    },
}

pub(super) struct TestIo {
    fail_rename: bool,
    remove: RemoveBehavior,
    remove_calls: Cell<usize>,
    started: Instant,
    clock: Cell<Instant>,
}

impl TestIo {
    pub(super) fn rename_failure() -> Self {
        Self::new(true, RemoveBehavior::Real)
    }

    pub(super) fn remove_failure() -> Self {
        Self::permanent_remove_failure(std::io::ErrorKind::PermissionDenied)
    }

    pub(super) fn permanent_remove_failure(kind: std::io::ErrorKind) -> Self {
        Self::new(false, RemoveBehavior::Fail(kind))
    }

    pub(super) fn transient_remove_failures(count: usize, kind: std::io::ErrorKind) -> Self {
        Self::new(
            false,
            RemoveBehavior::Transient {
                remaining: Cell::new(count),
                kind,
            },
        )
    }

    pub(super) fn remove_calls(&self) -> usize {
        self.remove_calls.get()
    }

    pub(super) fn elapsed(&self) -> Duration {
        self.clock.get().duration_since(self.started)
    }

    fn new(fail_rename: bool, remove: RemoveBehavior) -> Self {
        let now = Instant::now();
        Self {
            fail_rename,
            remove,
            remove_calls: Cell::default(),
            started: now,
            clock: Cell::new(now),
        }
    }
}

impl NamespaceRetirementIo for TestIo {
    fn rename(&self, source: &Path, target: &Path) -> std::io::Result<()> {
        if self.fail_rename {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                INJECTED_RENAME_FAILURE,
            ));
        }
        std::fs::rename(source, target)
    }

    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
        self.remove_calls.set(self.remove_calls.get() + 1);
        match &self.remove {
            RemoveBehavior::Real => std::fs::remove_dir_all(path),
            RemoveBehavior::Fail(kind) => Err(remove_error(*kind)),
            RemoveBehavior::Transient { remaining, kind } if remaining.get() > 0 => {
                remaining.set(remaining.get() - 1);
                Err(remove_error(*kind))
            }
            RemoveBehavior::Transient { .. } => std::fs::remove_dir_all(path),
        }
    }

    fn pause(&self, duration: Duration) {
        self.clock.set(self.clock.get() + duration);
    }

    fn now(&self) -> Instant {
        self.clock.get()
    }
}

fn remove_error(kind: std::io::ErrorKind) -> std::io::Error {
    std::io::Error::new(kind, INJECTED_REMOVE_FAILURE)
}
