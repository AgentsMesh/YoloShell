use super::{RunningHub, acquire, deadline};
use tokio::{sync::oneshot, task::JoinHandle};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseGrant, LeaseMode};

/// The test's independent controller follows its advertised renewal schedule.
pub struct Holder {
    pub original: LeaseGrant,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<LeaseGrant>>,
}

impl Holder {
    pub async fn start(hub: &RunningHub, target: RuntimeTarget) -> Self {
        let mut client = hub.client().await;
        let reply = client
            .call(hub.hub, acquire(target, LeaseMode::Acquire {}), deadline())
            .await
            .unwrap();
        let Reply::Lease { grant: original } = reply else {
            panic!("external clipboard holder acquire returned {reply:?}")
        };
        let origin = hub.hub;
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut grant = original;
            loop {
                let renew_after = std::time::Duration::from_millis(grant.fields().renew_after_ms);
                tokio::select! {
                    _ = &mut stopped => return grant,
                    _ = tokio::time::sleep(renew_after) => {}
                }
                // A renewal failure ends the fixture; never reacquire or take over.
                let reply = client
                    .call(origin, Command::Renew { grant }, deadline())
                    .await
                    .expect("external clipboard holder renewal RPC");
                let Reply::Lease { grant: renewed } = reply else {
                    panic!("external clipboard holder renewal returned {reply:?}")
                };
                assert_eq!(renewed.fields().target, original.fields().target);
                assert_eq!(renewed.fields().principal, original.fields().principal);
                assert_eq!(renewed.fields().generation, original.fields().generation);
                assert_eq!(renewed.fields().capability, original.fields().capability);
                grant = renewed;
            }
        });
        Self {
            original,
            stop: Some(stop),
            task: Some(task),
        }
    }

    pub async fn stop(mut self) -> LeaseGrant {
        let _ = self.stop.take().unwrap().send(());
        // Join any admitted renewal before replacing the source runtime.
        let mut task = self.task.take().unwrap();
        match tokio::time::timeout(super::super::support::TEST_TIMEOUT, &mut task).await {
            Ok(result) => result.unwrap(),
            Err(error) => {
                task.abort();
                panic!("external clipboard holder did not finish renewal: {error}")
            }
        }
    }
}

impl Drop for Holder {
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
