use super::constants::NETWORK_COMPLETION_FAILURE;
use crate::{ManagementService, generation::Running};
use std::sync::{Arc, Weak};
use tokio::sync::oneshot;
use yolo_hub_federation::Federation;

/// Retains the actual stopped generation while its network completion is delayed.
/// Federation tests separately exercise protocol-service and event-source owners.
pub struct RetirementGate {
    pub stopped: oneshot::Receiver<()>,
    pub previous: Weak<Federation>,
    release: Option<oneshot::Sender<()>>,
}

impl RetirementGate {
    pub fn install(service: &mut ManagementService) -> Self {
        Self::install_completion(service, false)
    }

    pub fn install_failed(service: &mut ManagementService) -> Self {
        Self::install_completion(service, true)
    }

    fn install_completion(service: &mut ManagementService, fail: bool) -> Self {
        let Running {
            node,
            stop,
            task,
            finished,
        } = service.running.take().unwrap();
        let previous = Arc::downgrade(&node);
        let retained = node.clone();
        let (stopped, observed) = oneshot::channel();
        let (release, released) = oneshot::channel();
        let task = tokio::spawn(async move {
            let result = task
                .await
                .map_err(anyhow::Error::from)
                .and_then(|result| result);
            let _ = stopped.send(());
            let _ = released.await;
            drop(retained);
            if fail {
                result?;
                anyhow::bail!(NETWORK_COMPLETION_FAILURE);
            }
            result
        });
        service.running = Some(Running {
            node,
            stop,
            task,
            finished,
        });
        Self {
            stopped: observed,
            previous,
            release: Some(release),
        }
    }

    pub fn release(&mut self) {
        self.release.take().unwrap().send(()).unwrap();
    }
}
