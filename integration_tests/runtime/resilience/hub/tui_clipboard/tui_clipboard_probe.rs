use super::{Driver, constants::*, deadline};
use std::sync::{Arc, Mutex};
use tokio::{sync::Notify, task::JoinHandle, time::timeout_at};
#[path = "tui_clipboard_capture.rs"]
mod capture;
use capture::Capture;

#[derive(Clone, Copy, Debug)]
pub enum Phase {
    SourceOutput,
    ExplicitCopy,
    ProviderOutput,
    FocusLost,
    RuntimeReplaced,
}

pub struct Probe {
    capture: Arc<Mutex<Capture>>,
    changed: Arc<Notify>,
    task: JoinHandle<()>,
}

impl Probe {
    pub fn start(mut output: yolo_client::Attachment) -> Self {
        let capture = Arc::new(Mutex::new(Capture::default()));
        let changed = Arc::new(Notify::new());
        let captured = capture.clone();
        let published = changed.clone();
        let task = tokio::spawn(async move {
            loop {
                // This owner continuously drains the independent tap, including
                // while the test awaits other RPCs. Only bytes bound its storage.
                let next = output.next_event().await;
                let terminal = captured.lock().unwrap().record(next);
                published.notify_one();
                if terminal {
                    return;
                }
            }
        });
        Self {
            capture,
            changed,
            task,
        }
    }

    pub async fn assert_commands(&self, phase: Phase, expected: &[&[u8]], driver: &Driver) {
        if let Err(error) = self.check_commands(expected).await {
            let evidence = self.capture.lock().unwrap().evidence();
            panic!(
                "clipboard probe failed: phase={phase:?} error={error} {evidence}; {}",
                driver.diagnostic().await,
            );
        }
    }

    async fn check_commands(&self, expected: &[&[u8]]) -> Result<(), &'static str> {
        let ready = async {
            loop {
                let changed = self.changed.notified();
                {
                    let capture = self.capture.lock().unwrap();
                    capture.healthy()?;
                    if capture.complete_commands(expected) {
                        return Ok::<(), &'static str>(());
                    }
                }
                changed.await;
            }
        };
        timeout_at(deadline(), ready)
            .await
            .map_err(|_| CAPTURE_DEADLINE)??;
        let quiet = tokio::time::Instant::now() + QUIET_WINDOW;
        loop {
            let changed = self.changed.notified();
            self.capture.lock().unwrap().healthy()?;
            if timeout_at(quiet, changed).await.is_err() {
                return self.capture.lock().unwrap().assert_commands(expected);
            }
        }
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[path = "tui_clipboard_probe_tests.rs"]
mod tests;
