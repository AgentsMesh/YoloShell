use crate::hub::support::deadline;
use std::sync::{Arc, Mutex};
use tokio::{sync::Notify, task::JoinHandle, time::timeout_at};
#[path = "tui_driver_output_capture.rs"]
mod capture;
use capture::Capture;

pub(crate) struct Output {
    capture: Arc<Mutex<Capture>>,
    changed: Arc<Notify>,
    task: Option<JoinHandle<()>>,
}

impl Output {
    pub fn start(mut attachment: yolo_client::Attachment) -> Self {
        let capture = Arc::new(Mutex::new(Capture::default()));
        let changed = Arc::new(Notify::new());
        let captured = capture.clone();
        let published = changed.clone();
        let task = tokio::spawn(async move {
            loop {
                let message = attachment.next_event().await;
                let terminal = captured.lock().unwrap().record(message);
                published.notify_one();
                if terminal {
                    return;
                }
            }
        });
        Self {
            capture,
            changed,
            task: Some(task),
        }
    }

    pub async fn wait(
        &self,
        screen: &mut vt100::Parser,
        ready: impl Fn(&str) -> bool,
    ) -> Result<(), String> {
        let observed = timeout_at(deadline(), async {
            loop {
                let changed = self.changed.notified();
                let bytes = self.capture.lock().unwrap().take_output();
                screen.process(&bytes);
                let pending = {
                    let capture = self.capture.lock().unwrap();
                    if let Some(failure) = capture.failure {
                        return Err(format!(
                            "TUI driver output failed: {failure:?}; {}",
                            capture.evidence()
                        ));
                    }
                    let pending = capture.has_pending_output();
                    if ready(&screen.screen().contents()) {
                        return Ok(());
                    }
                    pending
                };
                if pending {
                    // Let the existing deadline run even under continuous output.
                    tokio::task::yield_now().await;
                    continue;
                }
                changed.await;
            }
        })
        .await;
        observed.unwrap_or_else(|_| {
            Err(format!(
                "TUI screen did not settle; {}; screen={:?}",
                self.evidence(),
                screen.screen().contents()
            ))
        })
    }

    pub fn evidence(&self) -> String {
        self.capture.lock().unwrap().evidence()
    }

    pub async fn close(mut self) {
        // Cancelling next_event drops its owned socket. EOF detaches this view;
        // it does not terminate the outer shell or reopen an observation stream.
        let task = self.task.take().unwrap();
        task.abort();
        if let Err(error) = task.await {
            assert!(
                error.is_cancelled(),
                "TUI driver output reader failed: {error}"
            );
        }
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

#[path = "tui_driver_output_tests.rs"]
mod tests;
