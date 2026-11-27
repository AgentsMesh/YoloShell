use super::*;

pub(crate) type ProviderProcessScanTaskCustody =
    Arc<Mutex<HashMap<process::ProcessIdentity, ProviderProcessScanTask>>>;

#[derive(Default)]
pub(crate) struct ProviderProcessScanTasks {
    pub(super) custody: ProviderProcessScanTaskCustody,
}

impl ProviderProcessScanTasks {
    pub(crate) fn custody(&self) -> ProviderProcessScanTaskCustody {
        self.custody.clone()
    }

    pub(crate) fn register(
        &self,
        root: process::ProcessIdentity,
        identity: Arc<()>,
        settled: Arc<std::sync::atomic::AtomicBool>,
        task: tokio::task::JoinHandle<()>,
    ) {
        lock_recover(&self.custody).insert(
            root,
            ProviderProcessScanTask {
                identity,
                settled,
                task,
            },
        );
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        lock_recover(&self.custody).len()
    }
}

impl Drop for ProviderProcessScanTasks {
    fn drop(&mut self) {
        lock_recover(&self.custody).clear();
    }
}

pub(crate) struct ProviderProcessScanTask {
    pub(super) identity: Arc<()>,
    settled: Arc<std::sync::atomic::AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for ProviderProcessScanTask {
    fn drop(&mut self) {
        if !self.settled.load(std::sync::atomic::Ordering::Acquire) {
            self.task.abort();
        }
    }
}

pub(crate) struct ProviderProcessScanCleanup {
    pub(super) entries: Arc<Mutex<HashMap<process::ProcessIdentity, ProviderProcessScanEntry>>>,
    pub(super) tasks: ProviderProcessScanTaskCustody,
    pub(super) root: process::ProcessIdentity,
    pub(super) identity: Arc<()>,
    pub(super) settled: Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for ProviderProcessScanCleanup {
    fn drop(&mut self) {
        self.settled
            .store(true, std::sync::atomic::Ordering::Release);
        remove_scan_entry(&self.entries, self.root, &self.identity);
        let mut tasks = lock_recover(&self.tasks);
        if tasks
            .get(&self.root)
            .is_some_and(|task| Arc::ptr_eq(&task.identity, &self.identity))
        {
            tasks.remove(&self.root);
        }
    }
}

pub(crate) fn remove_scan_entry(
    entries: &Mutex<HashMap<process::ProcessIdentity, ProviderProcessScanEntry>>,
    root: process::ProcessIdentity,
    identity: &Arc<()>,
) {
    let mut entries = lock_recover(entries);
    if entries
        .get(&root)
        .is_some_and(|entry| Arc::ptr_eq(&entry.identity, identity))
    {
        entries.remove(&root);
    }
}
