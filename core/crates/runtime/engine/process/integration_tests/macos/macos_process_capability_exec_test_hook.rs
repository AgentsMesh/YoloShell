use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex, MutexGuard, TryLockError};

const EXEC_HOOK_ALREADY_INSTALLED: &str = "process capability exec hook is already installed";
const EXEC_HOOK_OWNERSHIP_LOST: &str = "process capability exec hook ownership was lost";
const RESTORE_DESCRIPTORS_MISSING: &str = "restore fallback descriptors were not published";
const PAUSE_PARTICIPANTS: usize = 2;

static EXEC_HOOK: Mutex<Option<Arc<ExecPause>>> = Mutex::new(None);
static EXEC_HOOK_OWNERSHIP: Mutex<()> = Mutex::new(());

struct ExecPause {
    phase: PausePhase,
    reached: Barrier,
    released: Barrier,
    fail_restore: AtomicBool,
    descriptors: Mutex<Option<(libc::c_int, libc::c_int)>>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PausePhase {
    InheritanceEnabled,
    RestoreFallback,
}

pub(super) struct ExecPauseGuard {
    pause: Arc<ExecPause>,
    _ownership: MutexGuard<'static, ()>,
}

pub(super) fn install() -> ExecPauseGuard {
    install_phase(PausePhase::InheritanceEnabled)
}

pub(super) fn install_restore_failure() -> ExecPauseGuard {
    install_phase(PausePhase::RestoreFallback)
}

pub(super) fn installation_is_owned() -> bool {
    matches!(
        EXEC_HOOK_OWNERSHIP.try_lock(),
        Err(TryLockError::WouldBlock)
    )
}

fn install_phase(phase: PausePhase) -> ExecPauseGuard {
    let ownership = EXEC_HOOK_OWNERSHIP
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let pause = Arc::new(ExecPause {
        phase,
        reached: Barrier::new(PAUSE_PARTICIPANTS),
        released: Barrier::new(PAUSE_PARTICIPANTS),
        fail_restore: AtomicBool::new(phase == PausePhase::RestoreFallback),
        descriptors: Mutex::new(None),
    });
    let mut hook = EXEC_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(hook.is_none(), "{EXEC_HOOK_ALREADY_INSTALLED}");
    *hook = Some(Arc::clone(&pause));
    drop(hook);
    ExecPauseGuard {
        pause,
        _ownership: ownership,
    }
}

pub(super) fn pause_after_inheritance_enabled() {
    if let Some(pause) = installed(PausePhase::InheritanceEnabled) {
        pause.reached.wait();
        pause.released.wait();
    }
}

pub(super) fn fail_next_restore() -> bool {
    installed(PausePhase::RestoreFallback)
        .is_some_and(|pause| pause.fail_restore.swap(false, Ordering::SeqCst))
}

pub(super) fn pause_after_restore_fallback(original: libc::c_int, backup: libc::c_int) {
    if let Some(pause) = installed(PausePhase::RestoreFallback) {
        *pause
            .descriptors
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((original, backup));
        pause.reached.wait();
        pause.released.wait();
    }
}

fn installed(phase: PausePhase) -> Option<Arc<ExecPause>> {
    EXEC_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .filter(|pause| pause.phase == phase)
        .cloned()
}

impl ExecPauseGuard {
    pub(super) fn wait_until_reached(&self) {
        self.pause.reached.wait();
    }

    pub(super) fn release(&self) {
        self.pause.released.wait();
    }

    pub(super) fn descriptors(&self) -> (libc::c_int, libc::c_int) {
        self.pause
            .descriptors
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .expect(RESTORE_DESCRIPTORS_MISSING)
    }
}

impl Drop for ExecPauseGuard {
    fn drop(&mut self) {
        let installed = EXEC_HOOK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        assert!(
            installed.is_some_and(|pause| Arc::ptr_eq(&pause, &self.pause)),
            "{EXEC_HOOK_OWNERSHIP_LOST}"
        );
    }
}
