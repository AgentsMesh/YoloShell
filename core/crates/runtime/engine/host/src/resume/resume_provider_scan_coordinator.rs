use super::*;

pub(crate) const MAX_CONCURRENT_PROVIDER_PROCESS_SCANS: usize = 2;
pub(crate) const PROVIDER_PROCESS_SCAN_CHANNEL_CLOSED: &str =
    "provider resume process scan result channel closed";
pub(crate) const PROVIDER_PROCESS_SCAN_DEADLINE_EXPIRED: &str =
    "provider resume process scan deadline expired";

#[derive(Debug)]
pub(crate) enum ProviderProcessScanOutcome {
    Observed(crate::agent::ProcessScanObservation),
    Failed(String),
    JoinFailed(String),
}

#[derive(Clone)]
pub(crate) struct ProviderProcessScanEntry {
    pub(super) identity: Arc<()>,
    outcome: tokio::sync::watch::Receiver<Option<Arc<ProviderProcessScanOutcome>>>,
}

pub(crate) struct ProviderProcessScanCoordinator {
    gate: Arc<tokio::sync::Semaphore>,
    entries: Arc<Mutex<HashMap<process::ProcessIdentity, ProviderProcessScanEntry>>>,
    tasks: ProviderProcessScanTasks,
}

impl ProviderProcessScanCoordinator {
    pub(crate) fn new() -> Self {
        Self {
            gate: Arc::new(tokio::sync::Semaphore::new(
                MAX_CONCURRENT_PROVIDER_PROCESS_SCANS,
            )),
            entries: Arc::new(Mutex::new(HashMap::new())),
            tasks: ProviderProcessScanTasks::default(),
        }
    }

    pub(crate) async fn observe(
        &self,
        scanner: Arc<dyn crate::agent::ProcessScanner>,
        root: process::ProcessIdentity,
        deadline: HostOperationDeadline,
    ) -> Option<Arc<ProviderProcessScanOutcome>> {
        let mut entry = self.entry(scanner, root, deadline);
        let outcome = loop {
            if let Some(outcome) = entry.outcome.borrow_and_update().clone() {
                break outcome;
            }
            let Some(changed) = deadline.timeout(entry.outcome.changed()).await else {
                self.cancel(root, &entry);
                return None;
            };
            if changed.is_err() {
                break Arc::new(ProviderProcessScanOutcome::JoinFailed(
                    PROVIDER_PROCESS_SCAN_CHANNEL_CLOSED.to_owned(),
                ));
            }
        };
        self.remove_consumed(root, &entry);
        Some(outcome)
    }

    pub(crate) fn cancel(&self, root: process::ProcessIdentity, cancelled: &ProviderProcessScanEntry) {
        let mut entries = lock_recover(&self.entries);
        if entries
            .get(&root)
            .is_some_and(|entry| Arc::ptr_eq(&entry.identity, &cancelled.identity))
        {
            entries.remove(&root);
        }
        drop(entries);
        let mut tasks = lock_recover(&self.tasks.custody);
        if tasks
            .get(&root)
            .is_some_and(|task| Arc::ptr_eq(&task.identity, &cancelled.identity))
        {
            tasks.remove(&root);
        }
    }

    pub(crate) fn entry(
        &self,
        scanner: Arc<dyn crate::agent::ProcessScanner>,
        root: process::ProcessIdentity,
        deadline: HostOperationDeadline,
    ) -> ProviderProcessScanEntry {
        let mut entries = lock_recover(&self.entries);
        if let Some(entry) = entries.get(&root) {
            return entry.clone();
        }
        let (outcome, receiver) = tokio::sync::watch::channel(None);
        let entry = ProviderProcessScanEntry {
            identity: Arc::new(()),
            outcome: receiver,
        };
        entries.insert(root, entry.clone());
        drop(entries);
        let settled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (start, started) = tokio::sync::oneshot::channel();
        let task = Self::start(
            scanner,
            root,
            deadline,
            self.gate.clone(),
            self.entries.clone(),
            outcome,
            entry.identity.clone(),
            self.tasks.custody(),
            settled.clone(),
            started,
        );
        self.tasks
            .register(root, entry.identity.clone(), settled, task);
        let _ = start.send(());
        entry
    }

    pub(crate) fn start(
        scanner: Arc<dyn crate::agent::ProcessScanner>,
        root: process::ProcessIdentity,
        deadline: HostOperationDeadline,
        gate: Arc<tokio::sync::Semaphore>,
        entries: Arc<Mutex<HashMap<process::ProcessIdentity, ProviderProcessScanEntry>>>,
        outcome: tokio::sync::watch::Sender<Option<Arc<ProviderProcessScanOutcome>>>,
        identity: Arc<()>,
        tasks: ProviderProcessScanTaskCustody,
        settled: Arc<std::sync::atomic::AtomicBool>,
        started: tokio::sync::oneshot::Receiver<()>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let _cleanup = ProviderProcessScanCleanup {
                entries,
                tasks,
                root,
                identity,
                settled,
            };
            if started.await.is_err() {
                return;
            }
            let result = match deadline.timeout(gate.acquire_owned()).await {
                Some(Ok(permit)) => {
                    let _permit = permit;
                    match deadline
                        .timeout(scanner.scan(root, deadline.process_instant()))
                        .await
                    {
                        Some(Ok(scan)) => ProviderProcessScanOutcome::Observed(scan),
                        Some(Err(error)) => ProviderProcessScanOutcome::Failed(format!("{error:#}")),
                        None => ProviderProcessScanOutcome::Failed(
                            PROVIDER_PROCESS_SCAN_DEADLINE_EXPIRED.to_owned(),
                        ),
                    }
                }
                Some(Err(error)) => ProviderProcessScanOutcome::JoinFailed(error.to_string()),
                None => ProviderProcessScanOutcome::Failed(
                    PROVIDER_PROCESS_SCAN_DEADLINE_EXPIRED.to_owned(),
                ),
            };
            outcome.send_replace(Some(Arc::new(result)));
        })
    }

    pub(crate) fn remove_consumed(&self, root: process::ProcessIdentity, consumed: &ProviderProcessScanEntry) {
        let mut entries = lock_recover(&self.entries);
        if entries
            .get(&root)
            .is_some_and(|entry| Arc::ptr_eq(&entry.identity, &consumed.identity))
        {
            entries.remove(&root);
        }
    }

    #[cfg(test)]
    pub(crate) fn available_permits(&self) -> usize {
        self.gate.available_permits()
    }

    #[cfg(test)]
    pub(crate) fn entry_count(&self) -> usize {
        lock_recover(&self.entries).len()
    }

    #[cfg(test)]
    pub(crate) fn task_count(&self) -> usize {
        self.tasks.len()
    }
}
