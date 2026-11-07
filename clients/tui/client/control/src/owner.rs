use crate::{constants::*, publication::Publication, server};
use anyhow::{Context, Result};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
    time::{Instant, timeout_at},
};
use yolo_tui_tab::TabStore;

/// The caller retains its TabStore until final state has been saved.
/// Finish only after dropping that store and its companion creation journal.
pub struct ClientControl {
    publication: Option<Publication>,
    task: Option<JoinHandle<Result<()>>>,
    requested: oneshot::Receiver<Instant>,
    completion: Option<oneshot::Sender<()>>,
    deadline: Option<Instant>,
}

impl ClientControl {
    pub async fn bind(store: &TabStore) -> Result<Self> {
        let deadline = Instant::now() + CONTROL_REQUEST_TIMEOUT;
        let (publication, listener, token) = Publication::bind(store, deadline).await?;
        let (sender, requested) = oneshot::channel();
        let (completion, finished) = oneshot::channel();
        let task = tokio::spawn(server::serve(listener, token, sender, finished));
        Ok(Self {
            publication: Some(publication),
            task: Some(task),
            requested,
            completion: Some(completion),
            deadline: None,
        })
    }

    pub async fn detached(&mut self) -> Result<()> {
        let deadline = (&mut self.requested).await.context(CONTROL_OWNER_FAILED)?;
        self.deadline = Some(deadline);
        Ok(())
    }

    /// Withdraw routing and control ownership while the caller still holds its view-state lock.
    pub fn withdraw(&mut self) -> Result<()> {
        if let Some(mut publication) = self.publication.take() {
            publication.withdraw()?;
        }
        Ok(())
    }

    pub async fn finish(mut self) -> Result<()> {
        self.withdraw()?;
        let Some(mut task) = self.task.take() else {
            return Ok(());
        };
        if let Some(deadline) = self.deadline {
            if let Some(completion) = self.completion.take() {
                if completion.send(()).is_err() {
                    task.abort();
                    let _ = task.await;
                    anyhow::bail!(CONTROL_OWNER_FAILED);
                }
            }
            match timeout_at(deadline, &mut task).await {
                Ok(result) => result.context(CONTROL_OWNER_FAILED)?,
                Err(error) => {
                    task.abort();
                    let _ = task.await;
                    Err(error).context(CONTROL_DEADLINE)
                }
            }
        } else {
            if task.is_finished() {
                return task.await.context(CONTROL_OWNER_FAILED)?;
            }
            task.abort();
            let _ = task.await;
            Ok(())
        }
    }
}

impl Drop for ClientControl {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
