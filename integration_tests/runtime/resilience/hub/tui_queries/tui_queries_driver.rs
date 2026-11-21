use super::{RunningHub, constants::*, deadline};
use crate::{Harness, runtime_ref_from_view};
use yolo_runtime::{model::WriterLeaseGrant, worker::RuntimeRef};
#[path = "../federation/federation_tui_capture.rs"]
mod capture;
#[path = "../tui_clipboard/tui_clipboard_driver.rs"]
mod clipboard;
#[path = "../tui_creation/tui_creation_recovery_driver.rs"]
mod creation_recovery;
#[path = "../tui_driver/tui_driver_diagnostics.rs"]
mod diagnostics;
#[path = "../federation/federation_tui_lease.rs"]
mod lease;
#[path = "../tui_driver/tui_driver_output.rs"]
pub(crate) mod output;
#[path = "../tui_driver/tui_driver_selection.rs"]
mod selection;
#[path = "../tui_resize_driver.rs"]
mod resize;
// The reused capture module's launch helper also refers to this type.
use crate::hub::federation_support::RunningNode;

pub struct Driver {
    client: yolo_client::Client,
    target: RuntimeRef,
    grant: WriterLeaseGrant,
    renewal: lease::Renewal,
    output: output::Output,
    screen: vt100::Parser,
    complete: String,
    workspace: tempfile::TempDir,
    search_characters: usize,
}
impl Driver {
    pub fn workspace_path(&self) -> &std::path::Path {
        self.workspace.path()
    }
    pub fn screen(&self) -> &vt100::Screen {
        self.screen.screen()
    }
    pub async fn clear_filter(&mut self) {
        self.send(SEARCH).await;
        self.wait(|text| text.contains(SEARCH_STATUS)).await;
        let mut edit = vec![SEARCH_BACKSPACE; self.search_characters];
        edit.extend_from_slice(FINISH_SEARCH);
        self.send(&edit).await;
        self.search_characters = usize::default();
        self.wait(|text| !text.contains(SEARCH_STATUS)).await;
    }
    pub async fn start(host: &Harness, hub: &RunningHub, workspace: tempfile::TempDir) -> Self {
        let binary = crate::bazel_runfile(&std::env::var(TUI_BINARY_ENV).unwrap()).unwrap();
        let view = host.create_cat(DRIVER_NAME).await;
        let target = runtime_ref_from_view(&view);
        let client = host.client();
        let grant = client
            .acquire_writer(
                target,
                DRIVER_NAME,
                Some(yolo_runtime::lease::constants::MAX_TTL_MS),
            )
            .await
            .unwrap();
        let renewal = lease::Renewal::start(client.clone(), target, grant.clone());
        client
            .resize(
                target,
                grant.generation,
                &grant.writer_capability,
                COLUMNS,
                ROWS,
            )
            .await
            .unwrap();
        let output = client
            .attach(
                target.terminal_id,
                yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
            )
            .await
            .unwrap();
        let mut screen = vt100::Parser::new(ROWS, COLUMNS, SCROLLBACK);
        screen.process(&output.initial.snapshot.as_ref().unwrap().data);
        let output = output::Output::start(output);
        let (launch, complete) = capture::launch_at(
            &binary,
            &hub.socket,
            hub.hub,
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
            workspace,
            search_characters: usize::default(),
        };
        driver.send(launch.as_bytes()).await;
        // Inventory arrives on a separate channel. Mutations require the control
        // generation rendered in the header, even when session rows are visible.
        driver
            .wait(|text| {
                text.contains(TERMINAL_PICKER_READY_SCREEN)
                    && text.contains(FIRST_NAME)
                    && text.contains(CONTROL_READY_HEADER)
            })
            .await;
        driver.restore_bound_view().await;
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
    pub async fn wait(&mut self, ready: impl Fn(&str) -> bool) {
        if let Err(error) = self.output.wait(&mut self.screen, ready).await {
            panic!("{error}; {}", self.diagnostic().await);
        }
    }
    pub async fn focused(
        &mut self,
        hub: yolo_hub_identity::HubId,
        target: yolo_hub_identity::TerminalAddress,
        name: &str,
    ) {
        tokio::time::timeout_at(deadline(), async {
            loop {
                if let Ok(document) =
                    yolo_runtime::paths::read_json::<yolo_tui_tab::TabDocument>(
                        &self
                            .workspace
                            .path()
                            .join(yolo_tui_tab::TAB_FILE),
                    )
                {
                    document.validate(hub).unwrap();
                    let layout = &document.tabs.active().view.layout;
                    if layout
                        .panes()
                        .iter()
                        .any(|pane| pane.id == layout.focused() && pane.terminal == Some(target))
                    {
                        return;
                    }
                }
                tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
            }
        })
        .await
        .unwrap();
        // Drain the old frame without opening a geometry-changing picker.
        self.send(PREFIX).await;
        self.wait(|text| text.contains(PREFIX_HINT)).await;
        self.send(ESCAPE).await;
        let title = format!("{name}{CONTROL}");
        self.wait(|text| {
            text.lines()
                .last()
                .is_some_and(|line| line.contains(&title))
        })
        .await;
    }
    pub async fn finish(mut self) -> tempfile::TempDir {
        self.send(DETACH).await;
        let complete = self.complete.clone();
        self.wait(|text| text.contains(&complete)).await;
        self.output.close().await;
        let grant = self.renewal.stop().await;
        self.client
            .release_writer(&grant, self.target)
            .await
            .unwrap();
        self.client
            .terminate(self.target.terminal_id, false)
            .await
            .unwrap();
        self.workspace
    }
}
