pub(super) const CREATION_LEDGER_LOCK_POLL_INTERVAL: Duration = Duration::from_millis(1);
pub(super) const CREATION_PUBLICATION_DEADLINE_EXPIRED: &str =
    "creation deadline expired before durable publication";
pub(super) const CREATION_PUBLICATION_COMMIT_IN_FLIGHT: &str =
    "creation publication commit owns an uncertain outcome";
pub(super) const CREATION_LEDGER_POISON_RECOVERY_LOG: &str =
    "recovering poisoned creation ledger mutex";
pub(super) const WORKER_START_UNCERTAIN_LOG: &str =
    "worker crossed the spawn boundary but readiness is uncertain";
