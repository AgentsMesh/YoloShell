#[cfg(test)]
use super::identity::FileIdentity;
use super::identity::ObjectIdentity;
use anyhow::Result;
use std::collections::HashMap;
use std::fs::File;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};

const NO_HOLDERS: usize = 0;
const INITIAL_HOLDER_COUNT: usize = 1;
const HOLDER_COUNT_STEP: usize = 1;

#[path = "process_file_lock_kernel.rs"]
mod kernel;
#[path = "process_file_lock_drop.rs"]
mod lock_drop;
#[path = "process_file_lock_registry.rs"]
mod registry;
#[path = "process_file_lock_turn.rs"]
mod turn;
#[path = "process_file_lock_wait_signal.rs"]
mod wait_signal;

pub(crate) use turn::ProcessLockTurn;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Exclusive,
    Shared,
}

#[derive(Debug)]
pub struct ProcessFileLock {
    lease: LockLease,
    key: ObjectIdentity,
    mode: Mode,
    process_id: u32,
}

#[derive(Debug)]
enum LockLease {
    Kernel(Arc<KernelLease>),
    #[cfg(test)]
    Logical,
}

#[derive(Debug)]
struct KernelLease {
    descriptor: File,
    locked: AtomicBool,
    process_id: u32,
}

struct Entry {
    holders: usize,
    lease: EntryLease,
    mode: Mode,
}

enum EntryLease {
    Pending(Arc<wait_signal::ProcessWaitSignal>),
    Kernel(Weak<KernelLease>),
    #[cfg(test)]
    Logical,
}

struct Registry {
    entries: HashMap<ObjectIdentity, Entry>,
    turns: HashMap<ObjectIdentity, turn::TurnState>,
}

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

impl ProcessFileLock {
    #[cfg(test)]
    pub(crate) fn try_acquire(identity: FileIdentity, mode: Mode) -> Result<Option<Self>> {
        Self::try_acquire_object(identity.object_identity(), mode)
    }

    #[cfg(test)]
    fn try_acquire_object(identity: ObjectIdentity, mode: Mode) -> Result<Option<Self>> {
        registry::try_acquire_logical(identity, mode)
    }

    #[cfg(test)]
    fn detached_logical_for_test(key: ObjectIdentity, mode: Mode, process_id: u32) -> Self {
        Self {
            lease: LockLease::Logical,
            key,
            mode,
            process_id,
        }
    }

    pub(in crate::paths) fn holds_exclusive(&self, identity: ObjectIdentity) -> bool {
        self.process_id == std::process::id()
            && self.key == identity
            && self.mode == Mode::Exclusive
            && matches!(&self.lease, LockLease::Kernel(_))
    }
}

fn current_registry() -> MutexGuard<'static, Registry> {
    let mutex = REGISTRY.get_or_init(|| Mutex::new(Registry::new()));
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Registry {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            turns: HashMap::new(),
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process_file_lock/process_file_lock_cloexec_tests.rs"]
mod cloexec_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process_file_lock/process_file_lock_kernel_tests.rs"]
mod kernel_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process_file_lock/process_file_lock_ownership_tests.rs"]
mod ownership_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process_file_lock/process_file_lock_tests.rs"]
mod tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process_file_lock/process_file_lock_turn_admission_tests.rs"]
mod turn_admission_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process_file_lock/process_file_lock_turn_foreign_tests.rs"]
mod turn_foreign_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process_file_lock/process_file_lock_turn_notification_tests.rs"]
mod turn_notification_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process_file_lock/process_file_lock_turn_tests.rs"]
mod turn_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process_file_lock/process_file_lock_wait_signal_tests.rs"]
mod wait_signal_tests;
