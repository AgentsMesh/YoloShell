use crate::host::tests::*;
const PROVIDER_BLOCKING_SCAN_ENTRY_TIMEOUT_DETAIL: &str =
    "provider process scan did not enter the blocking scanner";
const PROVIDER_BLOCKING_SCAN_SETTLEMENT_TIMEOUT_DETAIL: &str =
    "provider process scan did not settle";

#[derive(Clone)]
enum ScriptedProviderScan {
    Stable(Vec<crate::agent::ObservedProcess>),
    ObservedAt(crate::agent::FencedDescendants, i64, i64),
    RootChanged,
    Failed,
    Blocking(Arc<BlockingProviderScan>),
    Sequence(Arc<std::sync::Mutex<std::collections::VecDeque<crate::agent::FencedDescendants>>>),
}

impl ScriptedProviderScan {
    fn sequence(scans: Vec<crate::agent::FencedDescendants>) -> Self {
        Self::Sequence(Arc::new(std::sync::Mutex::new(scans.into())))
    }
}

struct BlockingProviderScan {
    entered: std::sync::atomic::AtomicBool,
    settled: std::sync::atomic::AtomicBool,
    state_changed: tokio::sync::Notify,
    result: crate::agent::FencedDescendants,
}

impl BlockingProviderScan {
    fn new(result: crate::agent::FencedDescendants) -> Arc<Self> {
        Arc::new(Self {
            entered: std::sync::atomic::AtomicBool::new(false),
            settled: std::sync::atomic::AtomicBool::new(false),
            state_changed: tokio::sync::Notify::new(),
            result,
        })
    }

    async fn await_entry(&self) {
        self.await_entry_with(PROVIDER_WAIT_IMMEDIATE_BOUND).await;
    }

    async fn await_entry_with(&self, timeout: std::time::Duration) {
        self.await_state(
            &self.entered,
            timeout,
            PROVIDER_BLOCKING_SCAN_ENTRY_TIMEOUT_DETAIL,
        )
            .await;
    }

    async fn await_settlement(&self) {
        self.await_state(
            &self.settled,
            PROVIDER_WAIT_IMMEDIATE_BOUND,
            PROVIDER_BLOCKING_SCAN_SETTLEMENT_TIMEOUT_DETAIL,
        )
            .await;
    }

    async fn scan(&self) -> anyhow::Result<crate::agent::FencedDescendants> {
        let _settlement = BlockingProviderScanSettlement(self);
        self.entered
            .store(true, std::sync::atomic::Ordering::Release);
        self.state_changed.notify_waiters();
        std::future::pending::<()>().await;
        Ok(self.result.clone())
    }

    async fn await_state(
        &self,
        state: &std::sync::atomic::AtomicBool,
        timeout: std::time::Duration,
        detail: &str,
    ) {
        tokio::time::timeout(timeout, async {
            while !state.load(std::sync::atomic::Ordering::Acquire) {
                let changed = self.state_changed.notified();
                if state.load(std::sync::atomic::Ordering::Acquire) {
                    break;
                }
                changed.await;
            }
        })
        .await
        .expect(detail);
    }
}

struct BlockingProviderScanSettlement<'a>(&'a BlockingProviderScan);

impl Drop for BlockingProviderScanSettlement<'_> {
    fn drop(&mut self) {
        self.0
            .settled
            .store(true, std::sync::atomic::Ordering::Release);
        self.0.state_changed.notify_waiters();
    }
}

struct ScriptedProviderProcessScanner {
    calls: std::sync::atomic::AtomicUsize,
    scan: std::sync::Mutex<ScriptedProviderScan>,
}

impl ScriptedProviderProcessScanner {
    fn stable(processes: Vec<crate::agent::ObservedProcess>) -> Self {
        Self {
            calls: std::sync::atomic::AtomicUsize::new(COMPLETION_PROVIDER_NO_SCAN_CALLS),
            scan: std::sync::Mutex::new(ScriptedProviderScan::Stable(processes)),
        }
    }

    fn set_scan(&self, scan: ScriptedProviderScan) {
        *self.scan.lock().unwrap() = scan;
    }

    fn calls(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::Acquire)
    }

    async fn scripted_scan(
        &self,
        _deadline: std::time::Instant,
    ) -> anyhow::Result<crate::agent::ProcessScanObservation> {
        self.calls.fetch_add(
            COMPLETION_PROVIDER_EXPECTED_SCAN_CALLS,
            std::sync::atomic::Ordering::AcqRel,
        );
        let scan = self.scan.lock().unwrap().clone();
        let (descendants, started_at_ms, completed_at_ms) = match scan {
            ScriptedProviderScan::Stable(processes) => {
                completed_provider_scan(crate::agent::FencedDescendants::Stable(processes))
            }
            ScriptedProviderScan::ObservedAt(descendants, started_at_ms, completed_at_ms) => {
                (descendants, started_at_ms, completed_at_ms)
            }
            ScriptedProviderScan::RootChanged => {
                completed_provider_scan(crate::agent::FencedDescendants::RootChanged)
            }
            ScriptedProviderScan::Failed => {
                anyhow::bail!(COMPLETION_PROVIDER_PROCESS_SCAN_ERROR)
            }
            ScriptedProviderScan::Blocking(scan) => completed_provider_scan(scan.scan().await?),
            ScriptedProviderScan::Sequence(scans) => {
                let mut scans = scans.lock().unwrap();
                let descendants = if scans.len() > COMPLETION_PROVIDER_EXPECTED_SCAN_CALLS {
                    scans.pop_front().unwrap()
                } else {
                    scans
                        .front()
                        .cloned()
                        .unwrap_or(crate::agent::FencedDescendants::RootChanged)
                };
                completed_provider_scan(descendants)
            }
        };
        Ok(crate::agent::ProcessScanObservation {
            descendants,
            started_at_ms,
            completed_at_ms,
        })
    }
}

fn completed_provider_scan(
    descendants: crate::agent::FencedDescendants,
) -> (crate::agent::FencedDescendants, i64, i64) {
    let observed_at_ms = current_provider_scan_timestamp();
    (descendants, observed_at_ms, observed_at_ms)
}

fn current_provider_scan_timestamp() -> i64 {
    crate::model::now_ms().saturating_add(COMPLETION_PROVIDER_TIMESTAMP_INCREMENT)
}

impl crate::agent::ProcessScanner for ScriptedProviderProcessScanner {
    fn scan<'a>(
        &'a self,
        _: process::ProcessIdentity,
        deadline: std::time::Instant,
    ) -> crate::agent::ProcessScanFuture<'a> {
        Box::pin(self.scripted_scan(deadline))
    }
}
