use super::constants::*;
use crate::{
    Harness,
    hub::{
        support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY},
        tui_queries::driver::Driver,
        tui_workspaces::support,
    },
};
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_runtime::model::TerminalView;
use yolo_tui_tab::TabDocument;

pub(super) struct Fixture {
    pub host: Harness,
    pub driver: Driver,
    pub hub: RunningHub,
    pub root: tempfile::TempDir,
    pub baseline: TerminalView,
    pub target: RuntimeTarget,
    _driver_host: Harness,
}

impl Fixture {
    pub async fn start() -> Self {
        let host = Harness::start().await;
        let baseline = host.create_cat(FIRST_NAME).await;
        let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
        let hub = RunningHub::start(
            &host,
            &root.path().canonicalize().unwrap().join(STATE_DIRECTORY),
        )
        .await;
        let target = RuntimeTarget {
            address: TerminalAddress {
                hub_id: hub.hub,
                terminal_id: TerminalId::from_uuid(baseline.manifest.terminal_id).unwrap(),
            },
            runtime_epoch: RuntimeEpoch::from_uuid(
                baseline.runtime.as_ref().unwrap().runtime_epoch,
            )
            .unwrap(),
        };
        let driver_host = Harness::start().await;
        let mut driver = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
        driver.open(FIRST_NAME, CONTROL).await;
        let fixture = Self {
            host,
            driver,
            hub,
            root,
            baseline,
            target,
            _driver_host: driver_host,
        };
        support::document(&fixture.driver, &fixture.hub, |saved| {
            saved.tabs.entries().len() == ONE_TAB && binding(saved) == Some(target.address)
        })
        .await;
        fixture
    }

    pub async fn document(&self, matches: impl Fn(&TabDocument) -> bool) -> TabDocument {
        support::document(&self.driver, &self.hub, matches).await
    }

    pub async fn action(&mut self, bytes: &[u8]) {
        self.driver.send(PREFIX).await;
        self.driver.wait(|text| text.contains(PREFIX_HINT)).await;
        self.driver.send(bytes.strip_prefix(PREFIX).unwrap()).await;
    }

    pub async fn paste(&mut self, value: &str) {
        self.driver
            .send(format!("{PASTE_START}{value}{PASTE_END}").as_bytes())
            .await;
    }

    pub async fn writable(&mut self) {
        self.driver
            .wait(|text| {
                text.lines()
                    .last()
                    .is_some_and(|line| line.contains(CONTROL))
            })
            .await;
    }

    pub async fn marker(&mut self) {
        self.writable().await;
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let expected = format!("{MARKER_PREFIX}{nonce}");
        self.driver
            .send(format!("printf '{MARKER_PREFIX}%s\\n' '{nonce}'\r").as_bytes())
            .await;
        self.driver.wait(|text| text.contains(&expected)).await;
    }

    pub async fn baseline_is_live(&self) {
        support::runtime_unchanged(&self.host, self.target, &self.baseline).await;
    }

    pub async fn finish(self) {
        self.driver.finish().await;
        self.hub.stop().await;
    }
}

pub(super) fn binding(saved: &TabDocument) -> Option<TerminalAddress> {
    let layout = &saved.tabs.active().view.layout;
    layout
        .panes()
        .into_iter()
        .find(|pane| pane.id == layout.focused())
        .unwrap()
        .terminal
}

pub(super) fn empty(saved: &TabDocument) -> bool {
    saved.tabs.entries().len() == ONE_TAB
        && saved.tabs.active().view.layout.panes().len() == ONE_PANE
        && saved
            .tabs
            .active()
            .view
            .layout
            .panes()
            .iter()
            .all(|pane| pane.terminal.is_none())
}
