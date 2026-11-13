use crate::{constants::*, observation_admission::Admission, observation_mailbox::Mailbox};
use anyhow::{Result, ensure};
use std::sync::Arc;
use tokio::sync::{oneshot, watch};
use yolo_hub_federation_directory::ObservationHistory;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_observation_store::{OBSERVATION_DIRECTORY, ObservationStore, Retirement};
use yolo_hub_state::HubState;

struct Completion {
    mailbox: Arc<Mailbox>,
    finished: watch::Sender<bool>,
    admission: Option<Admission>,
}

impl Drop for Completion {
    fn drop(&mut self) {
        self.mailbox.retire();
        // serve has already dropped its store, including on unwind.
        drop(self.admission.take());
        self.finished.send_replace(true);
    }
}

pub(crate) fn run(
    open: impl FnOnce(Retirement) -> Result<(ObservationStore, ObservationHistory)>,
    mailbox: Arc<Mailbox>,
    ready: oneshot::Sender<ObservationHistory>,
    finished: watch::Sender<bool>,
    admission: Admission,
) {
    let _completion = Completion {
        mailbox: mailbox.clone(),
        finished,
        admission: Some(admission),
    };
    if let Err(error) = serve(open, &mailbox, ready) {
        tracing::warn!(%error, message = OBSERVATION_CACHE_UNAVAILABLE);
    }
}

pub(crate) fn open(
    state: Arc<HubState>,
    retirement: Retirement,
) -> Result<(ObservationStore, ObservationHistory)> {
    ensure!(!retirement.is_retired(), CLOSED);
    state.verify()?;
    ensure!(!retirement.is_retired(), CLOSED);
    let root = state.directory()?;
    ensure!(!retirement.is_retired(), CLOSED);
    let directory = root.ensure_descendant(&root.path().join(OBSERVATION_DIRECTORY))?;
    ensure!(!retirement.is_retired(), CLOSED);
    let store = ObservationStore::open_with_retirement(
        directory,
        state.peer_identity(),
        routing_limits(),
        retirement.clone(),
    )?;
    ensure!(!retirement.is_retired(), CLOSED);
    let history = store.history()?;
    ensure!(!retirement.is_retired(), CLOSED);
    state.verify()?;
    ensure!(!retirement.is_retired(), CLOSED);
    Ok((store, history))
}

fn serve(
    open: impl FnOnce(Retirement) -> Result<(ObservationStore, ObservationHistory)>,
    mailbox: &Mailbox,
    ready: oneshot::Sender<ObservationHistory>,
) -> Result<()> {
    let (mut store, history) = open(mailbox.retirement.clone())?;
    if !mailbox.activate() {
        return Ok(());
    }
    if ready.send(history).is_err() {
        return Ok(());
    }
    persist(mailbox, |history| store.record_history(history))
}

pub(crate) fn persist(
    mailbox: &Mailbox,
    mut record: impl FnMut(&ObservationHistory) -> Result<()>,
) -> Result<()> {
    while let Some(pending) = mailbox.next() {
        record(&pending.history)?;
        let _ = pending.completed.send(());
    }
    Ok(())
}
