use super::{
    federation_support::{RunningNode, deadline},
    management_support::ManagedNode,
};
use crate::{Harness, runtime_ref_from_view};
use yolo_runtime::{model::WriterLeaseGrant, worker::RuntimeRef};
#[path = "../federation/federation_tui_capture.rs"]
mod capture;
#[path = "../federation/federation_tui_lease.rs"]
mod lease;

const BINARY_ENV: &str = "YOLO_TEST_TUI_BINARY";
const DRIVER: &str = "Hub config driver";
const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";
pub const ENTER: &[u8] = b"\r";
pub const DOWN: &[u8] = b"\x1b[B";
pub const HOME: &[u8] = b"\x1b[H";
const ROOT_APPLY_STEPS: usize = 4;
pub const TAB: &[u8] = b"\t";
pub const ESC: &[u8] = b"\x1b";
pub const OPEN: &[u8] = b"\x02s\tm";
pub const DETAILS: &[u8] = b"I";
pub const SINGLE_AGENT_HEADER: &str = "Agent runs: 1";
pub const HOOK_SOURCE: &str = "Hooks";
pub const REOPEN_FROM_AGENTS: &[u8] = b"\tm";
pub const AGENTS: &[u8] = b"\x1b[Z";
pub const TOGGLE: &[u8] = b" ";
pub const QUIT: &[u8] = b"q";
pub const ACTIVE: &str = "Configuration active";
pub const TITLE: &str = "Hub Configuration";
pub const ADD_PEER: &str = "Add Connection";
pub const EDIT_PEER: &str = "Edit Connection";
pub const DRAFT: &str = "Unsaved configuration";
pub const CONFLICT: &str = "Hub configuration changed; reload is required";
pub const DISCARD: &str = "Discard Unsaved Configuration";

pub fn detail_contains(text: &str, expected: &str) -> bool {
    // The independent driver intentionally uses a 90-column terminal. Read
    // wrapped details from their own region instead of requiring a wide screen.
    text.lines()
        .map(|line| {
            line.chars()
                .skip(usize::from(
                    super::tui_workspaces::constants::SIDEBAR_COLUMNS,
                ))
                .collect::<String>()
        })
        .map(|line| line.trim_end().to_owned())
        .collect::<String>()
        .contains(expected)
}

pub struct Driver {
    client: yolo_client::Client,
    target: RuntimeRef,
    grant: WriterLeaseGrant,
    renewal: lease::Renewal,
    output: yolo_client::Attachment,
    screen: vt100::Parser,
    complete: String,
    _workspace: tempfile::TempDir,
}
impl Driver {
    pub async fn start(harness: &Harness, node: &ManagedNode) -> Self {
        let binary = crate::bazel_runfile(&std::env::var(BINARY_ENV).unwrap()).unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let session = harness.create_cat(DRIVER).await;
        let target = runtime_ref_from_view(&session);
        let client = harness.client();
        let grant = client
            .acquire_writer(
                target,
                DRIVER,
                Some(yolo_runtime::lease::constants::MAX_TTL_MS),
            )
            .await
            .unwrap();
        let renewal = lease::Renewal::start(client.clone(), target, grant.clone());
        let output = client
            .attach(
                session.manifest.terminal_id,
                yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
            )
            .await
            .unwrap();
        let snapshot = output.initial.snapshot.as_ref().unwrap();
        let mut screen = vt100::Parser::new(snapshot.rows, snapshot.cols, 0);
        screen.process(&snapshot.data);
        let (launch, complete) = capture::launch_at(
            &binary,
            &node.socket,
            node.hub,
            &workspace.path().canonicalize().unwrap(),
        );
        let mut driver = Self {
            client,
            target,
            grant,
            renewal,
            output,
            screen,
            complete,
            _workspace: workspace,
        };
        driver.send(launch.as_bytes()).await;
        driver.wait(|text| text.contains(DRIVER)).await;
        driver.send(OPEN).await;
        let listening = format!("Listening: {}", node.address);
        driver
            .wait(|text| text.contains(TITLE) && text.contains(ACTIVE) && text.contains(&listening))
            .await;
        driver
    }
    pub async fn send(&self, bytes: &[u8]) {
        self.client
            .input(
                self.target,
                self.grant.generation,
                &self.grant.writer_capability,
                bytes.to_vec(),
            )
            .await
            .unwrap();
    }
    pub async fn field(&self, value: &str) {
        let mut bytes = PASTE_START.to_vec();
        bytes.extend_from_slice(value.as_bytes());
        bytes.extend_from_slice(PASTE_END);
        bytes.extend_from_slice(ENTER);
        self.send(&bytes).await;
    }
    pub async fn wait(&mut self, ready: impl Fn(&str) -> bool) {
        capture::wait_screen(&mut self.output, &mut self.screen, ready).await;
    }
    pub async fn active(&mut self, revision: u64) {
        let label = format!("revision {revision}");
        self.wait(|text| text.contains(&label) && text.contains(ACTIVE))
            .await;
    }
    pub async fn root_apply(&mut self) {
        self.send(ESC).await;
        self.wait(|text| text.contains(DRAFT) && text.contains("Connections ("))
            .await;
        self.apply_from_root().await;
    }
    pub async fn apply_from_root(&self) {
        self.send(HOME).await;
        for _ in 0..ROOT_APPLY_STEPS {
            self.send(DOWN).await;
        }
        self.send(ENTER).await;
    }
    pub async fn finish(mut self) {
        self.send(ESC).await;
        self.wait(|text| text.contains("Hubs:")).await;
        self.send(QUIT).await;
        let complete = self.complete.clone();
        self.wait(|text| text.contains(&complete)).await;
        let grant = self.renewal.stop().await;
        self.client
            .release_writer(&grant, self.target)
            .await
            .unwrap();
        self.output.detach().await.unwrap();
    }
}
