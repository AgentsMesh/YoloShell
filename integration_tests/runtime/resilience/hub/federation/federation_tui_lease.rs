use super::deadline;
use tokio::{sync::oneshot, task::JoinHandle};
use yolo_runtime::{model::WriterLeaseGrant, worker::RuntimeRef};

/// Keeps only the test driver's outer PTY lease alive while the inner TUI runs.
/// It never acquires or renews the TUI's own origin-side writer authority.
pub struct Renewal {
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<WriterLeaseGrant>>,
}

impl Renewal {
    pub fn start(
        client: yolo_client::Client,
        target: RuntimeRef,
        original: WriterLeaseGrant,
    ) -> Self {
        let (stop, mut stopped) = oneshot::channel();
        let first_renewal =
            tokio::time::Instant::now() + std::time::Duration::from_millis(original.renew_after_ms);
        let task = tokio::spawn(async move {
            let mut grant = original.clone();
            let mut next_renewal = first_renewal;
            loop {
                tokio::select! {
                    _ = &mut stopped => return grant,
                    _ = tokio::time::sleep_until(next_renewal) => {}
                }
                let renewed = client
                    .renew_writer(&grant, target, Some(grant.ttl_ms))
                    .await
                    .expect(RENEWAL_FAILED);
                assert_eq!(renewed.holder_id, original.holder_id);
                assert_eq!(
                    renewed.controller_instance_id,
                    original.controller_instance_id
                );
                assert_eq!(renewed.generation, original.generation);
                assert_eq!(renewed.writer_capability, original.writer_capability);
                grant = renewed;
                next_renewal = tokio::time::Instant::now()
                    + std::time::Duration::from_millis(grant.renew_after_ms);
            }
        });
        Self {
            stop: Some(stop),
            task: Some(task),
        }
    }

    pub async fn stop(mut self) -> WriterLeaseGrant {
        let _ = self.stop.take().unwrap().send(());
        let task = self.task.as_mut().unwrap();
        let grant = tokio::time::timeout_at(deadline(), task)
            .await
            .expect(RENEWAL_DID_NOT_STOP)
            .expect(RENEWAL_FAILED);
        self.task.take();
        grant
    }
}

impl Drop for Renewal {
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

const RENEWAL_FAILED: &str = "TUI test driver lost its original writer lease";
const RENEWAL_DID_NOT_STOP: &str = "TUI test driver renewal did not stop";
