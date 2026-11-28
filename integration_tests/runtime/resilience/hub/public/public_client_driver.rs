use super::{constants::*, deadline, fixture::Fixture, launch};
use crate::{Harness, hub::tui_queries::driver::output::Output, runtime_ref_from_view};
use yolo_runtime::{model::WriterLeaseGrant, worker::RuntimeRef};
#[path = "../federation/federation_tui_lease.rs"]
pub(super) mod lease;

pub struct Driver {
    client: yolo_client::Client,
    target: RuntimeRef,
    grant: WriterLeaseGrant,
    renewal: lease::Renewal,
    output: Output,
    screen: vt100::Parser,
    complete: String,
    pub workspace: tempfile::TempDir,
}

impl Driver {
    pub async fn start(host: &Harness, fixture: &Fixture, workspace: tempfile::TempDir) -> Self {
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
        let attached = client
            .attach(
                target.terminal_id,
                yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
            )
            .await
            .unwrap();
        let mut screen = vt100::Parser::new(ROWS, COLUMNS, SCROLLBACK);
        screen.process(&attached.initial.snapshot.as_ref().unwrap().data);
        let output = Output::start(attached);
        let (command, complete) =
            launch::launch(fixture, &workspace.path().canonicalize().unwrap());
        let mut driver = Self {
            client,
            target,
            grant,
            renewal,
            output,
            screen,
            complete,
            workspace,
        };
        driver.send(command.as_bytes()).await;
        driver
            .wait(|text| text.contains(TERMINAL_PICKER_READY_SCREEN) && text.contains(CONTROL_READY_HEADER))
            .await;
        let saved: yolo_tui_tab::TabDocument = yolo_runtime::paths::read_json(
            &driver
                .workspace
                .path()
                .join(yolo_tui_tab::TAB_FILE),
        )
        .unwrap();
        if super::workspace::binding(&saved).is_some() {
            driver.send(ESCAPE).await;
        }
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
        self.wait_named("TUI screen", ready).await;
    }

    pub async fn wait_named(&mut self, label: &str, ready: impl Fn(&str) -> bool) {
        if let Err(error) = self.output.wait(&mut self.screen, ready).await {
            panic!(
                "{label}: {error}; remote TUI screen:\n{}",
                self.screen.screen().contents()
            );
        }
    }

    pub async fn open(&mut self, address: yolo_hub_identity::TerminalAddress, name: &str) {
        self.send(INVENTORY).await;
        self.wait(|text| text.contains(TERMINAL_PICKER_READY_SCREEN)).await;
        self.send(SEARCH).await;
        self.wait(|text| text.contains(SEARCH_STATUS)).await;
        let mut edit = address.to_string().into_bytes();
        edit.extend_from_slice(SELECT_SEARCH_RESULT);
        self.send(&edit).await;
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
