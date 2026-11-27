use super::*;
use std::os::fd::AsRawFd;
use std::sync::atomic::Ordering;

impl Drop for ProcessFileLock {
    fn drop(&mut self) {
        if self.process_id != std::process::id() {
            return;
        }
        let mut registry = current_registry();
        let remove = registry
            .entries
            .get_mut(&self.key)
            .is_some_and(|entry| release_matching_entry(entry, self));
        if remove {
            registry.entries.remove(&self.key);
        }
    }
}

fn release_matching_entry(entry: &mut Entry, lock: &ProcessFileLock) -> bool {
    if entry.mode != lock.mode
        || entry.holders == NO_HOLDERS
        || !lease_matches(&entry.lease, &lock.lease)
    {
        return false;
    }
    entry.holders -= HOLDER_COUNT_STEP;
    if entry.holders != NO_HOLDERS {
        return false;
    }
    match &lock.lease {
        LockLease::Kernel(lease) => lease.unlock(),
        #[cfg(test)]
        LockLease::Logical => {}
    }
    true
}

fn lease_matches(entry: &EntryLease, lock: &LockLease) -> bool {
    match (entry, lock) {
        (EntryLease::Kernel(entry), LockLease::Kernel(lock)) => entry
            .upgrade()
            .is_some_and(|entry| Arc::ptr_eq(&entry, lock)),
        #[cfg(test)]
        (EntryLease::Logical, LockLease::Logical) => true,
        _ => false,
    }
}

impl Drop for KernelLease {
    fn drop(&mut self) {
        self.unlock();
    }
}

impl KernelLease {
    fn unlock(&self) {
        if self.process_id == std::process::id() && self.locked.swap(false, Ordering::AcqRel) {
            kernel::unlock(self.descriptor.as_raw_fd());
        }
    }
}
