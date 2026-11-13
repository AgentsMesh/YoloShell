use super::*;
use anyhow::{Result, ensure};

const PENDING_RESERVATION_CHANGED: &str = "process file lock reservation changed";

pub(super) enum KernelAcquisition {
    Acquired(ProcessFileLock),
    Conflict,
    PendingShared(wait_signal::ProcessWaitToken),
    Reserved(KernelReservation),
}

pub(super) struct KernelReservation {
    active: bool,
    key: ObjectIdentity,
    mode: Mode,
    pending: Arc<wait_signal::ProcessWaitSignal>,
    process_id: u32,
}

pub(super) fn begin_kernel(key: ObjectIdentity, mode: Mode) -> Result<KernelAcquisition> {
    let process_id = std::process::id();
    let mut registry = current_registry();
    if let Some(entry) = registry.entries.get_mut(&key) {
        return Ok(existing_acquisition(entry, key, mode, process_id));
    }
    let pending = Arc::new(wait_signal::ProcessWaitSignal::default());
    registry.entries.insert(
        key,
        Entry {
            holders: NO_HOLDERS,
            lease: EntryLease::Pending(Arc::clone(&pending)),
            mode,
        },
    );
    Ok(KernelAcquisition::Reserved(KernelReservation {
        active: true,
        key,
        mode,
        pending,
        process_id,
    }))
}

fn existing_acquisition(
    entry: &mut Entry,
    key: ObjectIdentity,
    mode: Mode,
    process_id: u32,
) -> KernelAcquisition {
    if entry.mode != Mode::Shared || mode != Mode::Shared {
        return KernelAcquisition::Conflict;
    }
    match &entry.lease {
        EntryLease::Pending(pending) => KernelAcquisition::PendingShared(pending.token()),
        EntryLease::Kernel(lease) => {
            let Some(lease) = lease.upgrade() else {
                return KernelAcquisition::Conflict;
            };
            let Some(holders) = entry.holders.checked_add(HOLDER_COUNT_STEP) else {
                return KernelAcquisition::Conflict;
            };
            entry.holders = holders;
            KernelAcquisition::Acquired(ProcessFileLock {
                lease: LockLease::Kernel(lease),
                key,
                mode,
                process_id,
            })
        }
        #[cfg(test)]
        EntryLease::Logical => KernelAcquisition::Conflict,
    }
}

impl KernelReservation {
    pub(super) fn install(mut self, descriptor: File) -> Result<ProcessFileLock> {
        ensure!(
            self.process_id == std::process::id(),
            PENDING_RESERVATION_CHANGED
        );
        let lease = Arc::new(KernelLease {
            descriptor,
            locked: AtomicBool::new(true),
            process_id: self.process_id,
        });
        let mut registry = current_registry();
        let entry = registry
            .entries
            .get_mut(&self.key)
            .ok_or_else(|| anyhow::anyhow!(PENDING_RESERVATION_CHANGED))?;
        ensure!(
            entry.holders == NO_HOLDERS
                && entry.mode == self.mode
                && matches!(&entry.lease, EntryLease::Pending(pending) if Arc::ptr_eq(pending, &self.pending)),
            PENDING_RESERVATION_CHANGED
        );
        entry.holders = INITIAL_HOLDER_COUNT;
        entry.lease = EntryLease::Kernel(Arc::downgrade(&lease));
        self.active = false;
        drop(registry);
        self.pending.notify_all();
        Ok(ProcessFileLock {
            lease: LockLease::Kernel(lease),
            key: self.key,
            mode: self.mode,
            process_id: self.process_id,
        })
    }
}

impl Drop for KernelReservation {
    fn drop(&mut self) {
        if !self.active || self.process_id != std::process::id() {
            return;
        }
        let mut registry = current_registry();
        if registry.entries.get(&self.key).is_some_and(|entry| {
            entry.mode == self.mode
                && matches!(&entry.lease, EntryLease::Pending(pending) if Arc::ptr_eq(pending, &self.pending))
        }) {
            registry.entries.remove(&self.key);
        }
        drop(registry);
        self.pending.notify_all();
    }
}

#[cfg(test)]
pub(super) fn try_acquire_logical(
    key: ObjectIdentity,
    mode: Mode,
) -> Result<Option<ProcessFileLock>> {
    let process_id = std::process::id();
    let mut registry = current_registry();
    if let Some(entry) = registry.entries.get_mut(&key) {
        if entry.mode != Mode::Shared
            || mode != Mode::Shared
            || !matches!(entry.lease, EntryLease::Logical)
        {
            return Ok(None);
        }
        let Some(holders) = entry.holders.checked_add(HOLDER_COUNT_STEP) else {
            return Ok(None);
        };
        entry.holders = holders;
    } else {
        registry.entries.insert(
            key,
            Entry {
                holders: INITIAL_HOLDER_COUNT,
                lease: EntryLease::Logical,
                mode,
            },
        );
    }
    Ok(Some(ProcessFileLock {
        lease: LockLease::Logical,
        key,
        mode,
        process_id,
    }))
}
