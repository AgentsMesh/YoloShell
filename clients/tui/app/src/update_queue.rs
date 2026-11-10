use crate::{State, network::Job, network::Update};
use tokio::sync::mpsc;

pub(crate) struct Queue {
    normal: mpsc::UnboundedSender<Update>,
    priority: mpsc::UnboundedSender<Update>,
}

pub(crate) struct Forwarder {
    normal: mpsc::UnboundedReceiver<Update>,
    priority: mpsc::UnboundedReceiver<Update>,
}

impl Queue {
    pub(crate) fn new() -> (Self, Forwarder) {
        let (normal, normal_receiver) = mpsc::unbounded_channel();
        let (priority, priority_receiver) = mpsc::unbounded_channel();
        (
            Self { normal, priority },
            Forwarder {
                normal: normal_receiver,
                priority: priority_receiver,
            },
        )
    }

    pub(crate) fn normal(&self, update: Update) {
        let _ = self.normal.send(update);
    }

    pub(crate) fn priority(&self, update: Update) {
        let _ = self.priority.send(update);
    }
}

impl Forwarder {
    pub(crate) async fn run(self, normal: mpsc::Sender<Update>, priority: mpsc::Sender<Update>) {
        tokio::join!(
            forward(self.normal, normal),
            forward(self.priority, priority)
        );
    }
}

async fn forward(mut updates: mpsc::UnboundedReceiver<Update>, sender: mpsc::Sender<Update>) {
    while let Some(update) = updates.recv().await {
        if sender.send(update).await.is_err() {
            break;
        }
    }
}

/// Applies queued updates before the next redraw. Priority updates are drained
/// first so coalescing cannot starve control replies.
pub(crate) fn drain_bounded(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    priority: &mut mpsc::Receiver<Update>,
    normal: &mut mpsc::Receiver<Update>,
    observations: &mut mpsc::Receiver<Update>,
    budget: usize,
) {
    let mut coalesced = 0;
    for receiver in [priority, normal, observations] {
        while coalesced < budget {
            match receiver.try_recv() {
                Ok(update) => {
                    crate::events::apply(state, jobs, update);
                    coalesced += 1;
                }
                Err(_) => break,
            }
        }
    }
}
