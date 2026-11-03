use super::{constants::*, deadline};
use crate::{Harness, runtime_ref_from_view};
use std::path::Path;
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_terminal_query_fixture::{Command, Report, constants as fixture};
#[path = "tui_queries_diagnostics.rs"]
mod diagnostics;

pub struct Source {
    pub name: &'static str,
    pub target: RuntimeTarget,
    root: tempfile::TempDir,
    sequence: u64,
    client: yolo_client::Client,
}

impl Source {
    pub async fn start(host: &Harness, hub: yolo_hub_identity::HubId, name: &'static str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let binary = crate::bazel_runfile(&std::env::var(fixture::BINARY_ENV).unwrap()).unwrap();
        let view = host.create_cat(name).await;
        let runtime = runtime_ref_from_view(&view);
        let client = host.client();
        let grant = client
            .acquire_writer(
                runtime,
                name,
                Some(yolo_runtime::lease::constants::MAX_TTL_MS),
            )
            .await
            .unwrap();
        let command = format!(
            "exec '{}' '{}'\n",
            quote(&binary),
            quote(&root.path().canonicalize().unwrap())
        );
        client
            .input(
                runtime,
                grant.generation,
                &grant.writer_capability,
                command.into_bytes(),
            )
            .await
            .unwrap();
        tokio::time::timeout_at(deadline(), async {
            while std::fs::read(root.path().join(fixture::READY_FILE))
                .ok()
                .as_deref()
                != Some(fixture::READY_VALUE)
            {
                tokio::time::sleep(fixture::POLL).await;
            }
        })
        .await
        .unwrap();
        client.release_writer(&grant, runtime).await.unwrap();
        Self {
            name,
            target: RuntimeTarget {
                address: TerminalAddress {
                    hub_id: hub,
                    terminal_id: TerminalId::from_uuid(runtime.terminal_id).unwrap(),
                },
                runtime_epoch: RuntimeEpoch::from_uuid(runtime.runtime_epoch).unwrap(),
            },
            root,
            sequence: u64::default(),
            client,
        }
    }

    pub async fn emit(&mut self, output: &[u8]) -> Vec<u8> {
        self.sequence += fixture::SEQUENCE_STEP;
        eprintln!(
            "terminal query command: source={} sequence={} output={output:?}",
            self.name, self.sequence
        );
        let command = Command {
            sequence: self.sequence,
            output: output.to_vec(),
            settle_ms: SETTLE_MS,
        };
        yolo_runtime::paths::atomic_write_json(
            &self.root.path().join(fixture::COMMAND_FILE),
            &command,
        )
        .unwrap();
        let result = tokio::time::timeout_at(deadline(), async {
            loop {
                if let Ok(bytes) = std::fs::read(self.root.path().join(fixture::REPORT_FILE)) {
                    if let Ok(report) = serde_json::from_slice::<Report>(&bytes) {
                        if report.sequence == self.sequence {
                            return report.received;
                        }
                    }
                }
                tokio::time::sleep(fixture::POLL).await;
            }
        })
        .await;
        match result {
            Ok(received) => received,
            Err(_) => panic!(
                "fixture command report deadline: {}",
                self.diagnostic().await
            ),
        }
    }

    pub async fn expect(&mut self, output: &[u8], expected: &[u8]) {
        eprintln!(
            "terminal query step: source={}, sequence={}, output={output:?}, expected={expected:?}",
            self.name,
            self.sequence + fixture::SEQUENCE_STEP
        );
        let result = tokio::time::timeout_at(deadline(), async {
            let mut received = self.emit(output).await;
            loop {
                assert!(
                    expected.starts_with(&received),
                    "query reply mismatch for {} sequence {}: received {received:?}, expected {expected:?}",
                    self.name, self.sequence
                );
                if received == expected {
                    return;
                }
                received = self.emit(EMPTY_OUTPUT).await;
            }
        })
        .await;
        assert!(
            result.is_ok(),
            "terminal query response deadline: output={output:?}, expected={expected:?}; {}",
            self.diagnostic().await
        );
    }
}

fn quote(path: &Path) -> String {
    path.to_string_lossy().replace('\'', QUOTE_ESCAPE)
}
