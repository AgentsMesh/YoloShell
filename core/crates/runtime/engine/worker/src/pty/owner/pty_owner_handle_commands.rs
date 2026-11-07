use super::{
    CommandCompletion, DestructiveCapability, PtyOwnerCommand, PtyOwnerCommandFailure,
    PtyOwnerHandle, PtyTerminationSubmission,
};
use anyhow::Result;
use std::sync::Arc;

impl PtyOwnerHandle {
    pub(crate) fn runtime_capabilities(&self) -> Vec<String> {
        match &self.destructive {
            DestructiveCapability::Available => {
                vec![crate::protocol::capability::SAFE_PROCESS_TREE_TERMINATION.to_owned()]
            }
            DestructiveCapability::Unavailable(_) => Vec::new(),
        }
    }

    pub(crate) fn preflight_destructive(&self) -> Result<()> {
        match &self.destructive {
            DestructiveCapability::Available => Ok(()),
            DestructiveCapability::Unavailable(error) => Err(anyhow::Error::new(error.clone())),
        }
    }

    pub(crate) fn submit_termination(&self, force: bool) -> Result<PtyTerminationSubmission> {
        self.submit_termination_after(force, || {})
    }

    #[cfg(test)]
    pub(crate) fn submit_termination_after_for_test(
        &self,
        force: bool,
        before_send: impl FnOnce(),
    ) -> Result<PtyTerminationSubmission> {
        self.submit_termination_after(force, before_send)
    }

    pub(crate) fn submit_termination_after(
        &self,
        force: bool,
        before_send: impl FnOnce(),
    ) -> Result<PtyTerminationSubmission> {
        if self.settlement.get().is_some() {
            return Ok(PtyTerminationSubmission {
                completion: None,
                settlement: Arc::clone(&self.settlement),
            });
        }
        let (response, completion) = tokio::sync::oneshot::channel();
        before_send();
        if self
            .commands
            .send(PtyOwnerCommand::Terminate { force, response })
            .is_err()
        {
            return Ok(PtyTerminationSubmission {
                completion: None,
                settlement: Arc::clone(&self.settlement),
            });
        }
        Ok(PtyTerminationSubmission {
            completion: Some(completion),
            settlement: Arc::clone(&self.settlement),
        })
    }
}

impl PtyTerminationSubmission {
    pub(crate) async fn wait(self) -> Result<()> {
        let Some(mut response) = self.completion else {
            return into_result(self.settlement.wait().await);
        };
        tokio::select! {
            biased;
            received = &mut response => match received {
                Ok(completion) => into_result(completion),
                Err(_) => into_result(self.settlement.wait().await),
            },
            completion = self.settlement.wait() => into_result(completion),
        }
    }
}

pub(crate) fn into_result(completion: CommandCompletion) -> Result<()> {
    completion.map_err(|failure| match failure {
        PtyOwnerCommandFailure::DestructiveUnavailable(error) => anyhow::Error::new(error),
        PtyOwnerCommandFailure::CleanupDeferred(error) => anyhow::Error::new(error),
        PtyOwnerCommandFailure::Owner(detail) => anyhow::Error::msg(detail),
    })
}
