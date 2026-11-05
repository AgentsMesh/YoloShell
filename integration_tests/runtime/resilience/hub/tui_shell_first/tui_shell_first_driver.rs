use super::{RunningHub, constants::*, deadline};
use crate::hub::tui_queries::driver::output::Output;
use crate::{Harness, runtime_ref_from_view};
use std::path::{Path, PathBuf};
use yolo_runtime::{model::WriterLeaseGrant, worker::RuntimeRef};
#[path = "../federation/federation_tui_lease.rs"]
mod lease;

pub struct Driver {
    client: yolo_client::Client,
    target: RuntimeRef,
    grant: WriterLeaseGrant,
    renewal: lease::Renewal,
    output: Output,
    screen: vt100::Parser,
    binary: PathBuf,
    complete: Option<String>,
}

impl Driver {
    pub fn screen(&self) -> &vt100::Screen {
        self.screen.screen()
    }

    pub async fn new(host: &Harness) -> Self {
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
        let output = Output::start(output);
        Self {
            client,
            target,
            grant,
            renewal,
            output,
            screen,
            binary,
            complete: None,
        }
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

    pub async fn wait(&mut self, predicate: impl Fn(&str) -> bool) {
        self.output.wait(&mut self.screen, predicate).await.unwrap();
    }

    pub async fn launch(&mut self, hub: &RunningHub, state: &Path, command: &str) {
        assert!(self.complete.is_none());
        self.screen = vt100::Parser::new(ROWS, COLUMNS, SCROLLBACK);
        let nonce = uuid::Uuid::new_v4();
        let complete = format!("{COMPLETE_PREFIX}{nonce}");
        let launch = format!(
            "{}; printf '%s%s:%s\\n' '{COMPLETE_PREFIX}' '{nonce}' \"$?\"\r",
            self.command(hub, state, command)
        );
        self.complete = Some(complete);
        self.send(launch.as_bytes()).await;
    }

    pub async fn returned(&mut self) {
        let marker = self.complete.take().unwrap();
        self.wait(|text| text.contains(&marker)).await;
        assert!(
            self.screen
                .screen()
                .contents()
                .contains(&format!("{marker}:{SUCCESS_STATUS}")),
            "{}",
            self.screen.screen().contents()
        );
    }

    pub async fn execute(&mut self, hub: &RunningHub, state: &Path, command: &str) -> String {
        assert!(self.complete.is_none());
        let nonce = uuid::Uuid::new_v4();
        let begin = format!("{BEGIN_PREFIX}{nonce}");
        let end = format!("{COMPLETE_PREFIX}{nonce}");
        let command = format!(
            "printf '%s%s\\n' '{BEGIN_PREFIX}' '{nonce}'; {}; printf '%s%s:%s\\n' '{COMPLETE_PREFIX}' '{nonce}' \"$?\"\r",
            self.command(hub, state, command)
        );
        self.send(command.as_bytes()).await;
        self.wait(|text| text.contains(&end)).await;
        let contents = self.screen.screen().contents();
        assert!(
            contents.contains(&format!("{end}:{SUCCESS_STATUS}")),
            "{contents}"
        );
        contents
            .split_once(&begin)
            .unwrap()
            .1
            .split_once(&end)
            .unwrap()
            .0
            .to_owned()
    }

    fn command(&self, hub: &RunningHub, state: &Path, command: &str) -> String {
        format!(
            "{STATE_ENV}='{}' {INACTIVE_ENV}=1 '{}' {command} {SOCKET_ARGUMENT} '{}' {HUB_ARGUMENT} {}",
            quote(state),
            quote(&self.binary),
            quote(&hub.socket),
            hub.hub
        )
    }

    pub async fn close(self) {
        assert!(self.complete.is_none());
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
    }
}

fn quote(path: &Path) -> String {
    path.to_str().unwrap().replace('\'', QUOTE_ESCAPE)
}
