const INGRESS_TEST_BIND_FAILURE: &str = "injected Codex ingress bind failure";
const INGRESS_TEST_LISTENER_FAILURE: &str = "injected Codex listener failure";
const INGRESS_TEST_LISTENER_PANIC: &str = "injected Codex listener panic";
const INGRESS_TEST_COUNTER_INCREMENT: u64 = 1;
const INGRESS_TEST_SINGLE_FAULT_COUNT: usize = 1;
const INGRESS_TEST_BIND_FAILURE_DECREMENT: usize = 1;

#[derive(Clone, Copy)]
enum IngressListenerFault {
    Failure,
    Panic,
}

#[derive(Default)]
struct IngressTestState {
    listener_faults: std::collections::VecDeque<IngressListenerFault>,
    bind_failures: usize,
    publications: u64,
    restart_delays: Vec<std::time::Duration>,
}

#[derive(Default)]
struct IngressTestControl {
    state: Mutex<IngressTestState>,
    fault_signal: tokio::sync::Notify,
    progress_signal: tokio::sync::Notify,
}

impl IngressListenerFault {
    fn into_result(self) -> anyhow::Result<()> {
        match self {
            Self::Failure => anyhow::bail!(INGRESS_TEST_LISTENER_FAILURE),
            Self::Panic => panic!("{INGRESS_TEST_LISTENER_PANIC}"),
        }
    }
}

impl IngressTestControl {
    fn lock(&self) -> std::sync::MutexGuard<'_, IngressTestState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn push_listener_faults(&self, fault: IngressListenerFault, count: usize) {
        let mut state = self.lock();
        state.listener_faults.reserve(count);
        for _ in 0..count {
            state.listener_faults.push_back(fault);
        }
        drop(state);
        self.fault_signal.notify_one();
    }

    async fn next_listener_fault(&self) -> IngressListenerFault {
        loop {
            let notified = self.fault_signal.notified();
            if let Some(fault) = self.lock().listener_faults.pop_front() {
                return fault;
            }
            notified.await;
        }
    }

    fn queue_bind_failures(&self, count: usize) {
        self.lock().bind_failures = count;
    }

    fn fail_bind(&self) -> bool {
        let mut state = self.lock();
        let Some(remaining) = state
            .bind_failures
            .checked_sub(INGRESS_TEST_BIND_FAILURE_DECREMENT)
        else {
            return false;
        };
        state.bind_failures = remaining;
        true
    }

    fn record_publication(&self) {
        let mut state = self.lock();
        state.publications = state
            .publications
            .saturating_add(INGRESS_TEST_COUNTER_INCREMENT);
        drop(state);
        self.progress_signal.notify_one();
    }

    fn record_restart_delay(&self, delay: std::time::Duration) {
        self.lock().restart_delays.push(delay);
        self.progress_signal.notify_one();
    }

    fn publications(&self) -> u64 {
        self.lock().publications
    }

    fn restart_delays(&self) -> Vec<std::time::Duration> {
        self.lock().restart_delays.clone()
    }

    async fn wait_for_publications(&self, target: u64) {
        self.wait_for_progress(|state| state.publications >= target)
            .await;
    }

    async fn wait_for_restart_delays(&self, target: usize) {
        self.wait_for_progress(|state| state.restart_delays.len() >= target)
            .await;
    }

    async fn wait_for_progress(&self, complete: impl Fn(&IngressTestState) -> bool) {
        loop {
            let notified = self.progress_signal.notified();
            if complete(&self.lock()) {
                return;
            }
            notified.await;
        }
    }
}

impl CodexHookIngressSupervisor {
    pub(in crate::worker::codex_hook) fn fail_listener_for_test(&self) {
        self.inner.test_control.push_listener_faults(
            IngressListenerFault::Failure,
            INGRESS_TEST_SINGLE_FAULT_COUNT,
        );
    }

    pub(in crate::worker::codex_hook) fn panic_listener_for_test(&self) {
        self.inner
            .test_control
            .push_listener_faults(IngressListenerFault::Panic, INGRESS_TEST_SINGLE_FAULT_COUNT);
    }

    pub(in crate::worker::codex_hook) fn fail_rebinds_for_test(&self, count: usize) {
        self.inner.test_control.queue_bind_failures(count);
    }

    pub(in crate::worker::codex_hook) fn abort_listener_for_test(&self) -> bool {
        let listener = crate::worker::lock_recover(&self.inner.listener);
        let Some(listener) = listener.as_ref() else {
            return false;
        };
        listener.abort.abort();
        true
    }

    pub(in crate::worker::codex_hook) fn listener_publications_for_test(&self) -> u64 {
        self.inner.test_control.publications()
    }

    pub(in crate::worker::codex_hook) fn restart_delays_for_test(
        &self,
    ) -> Vec<std::time::Duration> {
        self.inner.test_control.restart_delays()
    }

    pub(in crate::worker::codex_hook) async fn wait_for_listener_publications_for_test(
        &self,
        target: u64,
    ) {
        self.inner.test_control.wait_for_publications(target).await;
    }

    pub(in crate::worker::codex_hook) async fn wait_for_restart_delays_for_test(
        &self,
        target: usize,
    ) {
        self.inner
            .test_control
            .wait_for_restart_delays(target)
            .await;
    }
}
