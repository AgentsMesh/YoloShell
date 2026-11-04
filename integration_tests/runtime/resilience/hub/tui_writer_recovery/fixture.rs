use super::constants::*;
use crate::hub::{
    support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY},
    tui_queries::{
        constants::{CONTROL, EMPTY_OUTPUT, FIRST_NAME},
        driver::Driver,
        source::Source,
    },
    tui_workspaces::support::runtime_unchanged,
};
use crate::{Harness, runtime_ref_from_view};
use yolo_runtime::{model::TerminalView, worker::RuntimeRef};

pub(super) struct Fixture {
    pub host: Harness,
    pub driver: Driver,
    pub source: Source,
    pub original: TerminalView,
    root: tempfile::TempDir,
    hub: Option<RunningHub>,
    _driver_host: Harness,
}

impl Fixture {
    pub async fn start() -> Self {
        let host = Harness::start().await;
        let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
        let state = root.path().canonicalize().unwrap().join(STATE_DIRECTORY);
        let hub = RunningHub::start(&host, &state).await;
        let mut source = Source::start(&host, hub.hub, FIRST_NAME).await;
        let original = host
            .client()
            .get_session(source.target.address.terminal_id.as_uuid())
            .await
            .unwrap();
        let driver_host = Harness::start().await;
        let mut driver = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
        driver.open(source.name, CONTROL).await;
        driver.send(BASELINE_INPUT).await;
        source.expect(EMPTY_OUTPUT, BASELINE_INPUT).await;
        let mut fixture = Self {
            host,
            driver,
            source,
            original,
            root,
            hub: Some(hub),
            _driver_host: driver_host,
        };
        fixture.confirm_idle_input().await;
        fixture
    }

    pub fn runtime(&self) -> RuntimeRef {
        runtime_ref_from_view(&self.original)
    }

    pub async fn interrupt_hub(&mut self) {
        self.hub.take().unwrap().stop().await;
        self.wait_status(HUB_RECONNECTING).await;
        // Keys accepted by the outer PTY during an outage are not deferred commands.
        self.driver.send(OUTAGE_INPUT).await;
        assert_eq!(self.source.emit(EMPTY_OUTPUT).await, BASELINE_INPUT);
        self.assert_runtime().await;
    }

    pub async fn restart_hub(&mut self) {
        let state = self
            .root
            .path()
            .canonicalize()
            .unwrap()
            .join(STATE_DIRECTORY);
        let restored = RunningHub::start(&self.host, &state).await;
        assert_eq!(restored.hub, self.source.target.address.hub_id);
        self.hub = Some(restored);
    }

    pub async fn recovered(&mut self) {
        // No picker navigation, rebinding, take-control or application restart.
        self.wait_status(&format!("{}{CONTROL}", self.source.name))
            .await;
        self.driver.send(RECOVERED_INPUT).await;
        let expected = [BASELINE_INPUT, RECOVERED_INPUT].concat();
        self.source.expect(RECONNECTED_OUTPUT, &expected).await;
        self.driver
            .wait(|text| text.contains(RECONNECTED_MARKER))
            .await;
        assert_eq!(self.source.emit(EMPTY_OUTPUT).await, expected);
        self.assert_runtime().await;
    }

    async fn assert_runtime(&self) {
        runtime_unchanged(&self.host, self.source.target, &self.original).await;
    }

    async fn confirm_idle_input(&mut self) {
        // Receiving bytes in the source PTY is not an acknowledgement barrier
        // in the TUI. Before the outage, release through the same serial lane,
        // observe its completion remotely, then await a fresh confirmed grant.
        // No release/acquire/navigation is permitted after the outage begins.
        self.driver.send(RELEASE_INPUT).await;
        let runtime = self.runtime();
        let client = self.host.client();
        let released = tokio::time::timeout(RECOVERY_TIMEOUT, async {
            loop {
                if client.get_writer_lease(runtime).await.unwrap().is_none() {
                    break;
                }
                tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
            }
        })
        .await;
        assert!(
            released.is_ok(),
            "baseline release did not settle: {}",
            self.driver.diagnostic().await
        );
        self.driver.send(ACQUIRE_INPUT).await;
        self.wait_status(&format!("{}{CONTROL}", self.source.name))
            .await;
    }

    pub async fn wait_status(&mut self, expected: &str) {
        if tokio::time::timeout(
            RECOVERY_TIMEOUT,
            self.driver.wait(|text| text.contains(expected)),
        )
        .await
        .is_err()
        {
            let (driver, source) = tokio::join!(self.driver.diagnostic(), self.source.diagnostic());
            panic!(
                "writer recovery did not show {expected:?} within {RECOVERY_TIMEOUT:?}; {driver}; {source}"
            );
        }
    }

    pub async fn finish(self) {
        self.driver.finish().await;
        self.hub.unwrap().stop().await;
    }
}
