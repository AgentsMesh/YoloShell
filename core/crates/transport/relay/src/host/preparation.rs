use super::super::constants::HOST_ALREADY_RUNNING_CODE;
use super::{ClientError, RelayError, RelayRuntime};
use crate::model::{ExitReason, ExitRecord, TerminalRuntimeStatus, TerminalView};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PreparedSession {
    Active,
    Resumed,
    UserTerminated,
    Completed {
        runtime_epoch: Uuid,
        expected_exit: ExitRecord,
    },
}

impl PreparedSession {
    pub(crate) fn into_completed(self) -> Option<(Uuid, ExitRecord)> {
        match self {
            Self::Completed {
                runtime_epoch,
                expected_exit,
            } => Some((runtime_epoch, expected_exit)),
            Self::Active | Self::Resumed | Self::UserTerminated => None,
        }
    }

    pub(in crate::relay) fn was_resumed(&self) -> bool {
        matches!(self, Self::Resumed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SessionPlan {
    Attach(PreparedSession),
    Resume,
    PendingFinalization,
    Archived,
}

impl RelayRuntime {
    pub(super) async fn ensure_session_attachable(
        &self,
        id: Uuid,
    ) -> Result<PreparedSession, RelayError> {
        let view = self
            .client
            .get_session(id)
            .await
            .map_err(super::observation_error)?;
        match session_plan(&view) {
            SessionPlan::Attach(prepared) => Ok(prepared),
            SessionPlan::Archived => Err(RelayError::ArchivedSession(id)),
            SessionPlan::PendingFinalization => Err(RelayError::SessionNotAttachable(id)),
            SessionPlan::Resume => self.resume_attachable(id).await,
        }
    }

    async fn resume_attachable(&self, id: Uuid) -> Result<PreparedSession, RelayError> {
        match self.client.resume(id, false).await {
            Ok(resumed) if session_plan(&resumed) == active_plan() => Ok(PreparedSession::Resumed),
            Ok(_) => Err(RelayError::SessionNotAttachable(id)),
            Err(error) if is_resume_race(&error) => {
                let current = self
                    .client
                    .get_session(id)
                    .await
                    .map_err(super::observation_error)?;
                (session_plan(&current) == active_plan())
                    .then_some(PreparedSession::Active)
                    .ok_or(RelayError::SessionNotAttachable(id))
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(super) async fn ensure_same_completed_attachment(
        &self,
        id: Uuid,
        runtime_epoch: Uuid,
        expected_exit: &ExitRecord,
    ) -> Result<(), RelayError> {
        let view = self
            .client
            .get_session(id)
            .await
            .map_err(super::observation_error)?;
        match session_plan(&view) {
            SessionPlan::Attach(PreparedSession::Completed {
                runtime_epoch: actual_epoch,
                expected_exit: actual_exit,
            }) if actual_epoch == runtime_epoch && actual_exit == *expected_exit => Ok(()),
            _ => Err(RelayError::SessionNotAttachable(id)),
        }
    }
}

pub(super) fn session_plan(view: &TerminalView) -> SessionPlan {
    if view.manifest.archived {
        return SessionPlan::Archived;
    }
    if let Some((runtime_epoch, expected_exit)) = exact_completed_attachment(view) {
        return SessionPlan::Attach(PreparedSession::Completed {
            runtime_epoch,
            expected_exit,
        });
    }
    if view.completed_attachment.is_some() {
        return SessionPlan::PendingFinalization;
    }
    match view.runtime.as_ref() {
        Some(runtime)
            if matches!(
                runtime.status,
                TerminalRuntimeStatus::Running | TerminalRuntimeStatus::Starting
            ) =>
        {
            active_plan()
        }
        Some(runtime) if matches!(runtime.status, TerminalRuntimeStatus::Exited | TerminalRuntimeStatus::Lost) => {
            exited_plan(runtime)
        }
        Some(runtime) if runtime.status == TerminalRuntimeStatus::Unreachable => SessionPlan::Resume,
        Some(_) => SessionPlan::PendingFinalization,
        None => SessionPlan::Resume,
    }
}

fn exact_completed_attachment(view: &TerminalView) -> Option<(Uuid, ExitRecord)> {
    let completed = view.completed_attachment?;
    let runtime = view.runtime.as_ref()?;
    let exit = runtime.exit.as_ref()?;
    (completed.runtime_epoch != Uuid::nil()
        && completed.runtime_epoch == runtime.runtime_epoch
        && runtime.status == TerminalRuntimeStatus::Exited
        && exit.runtime_epoch == runtime.runtime_epoch
        && exit.reason == ExitReason::Natural
        && runtime.last_output_sequence == exit.last_output_sequence)
        .then(|| (completed.runtime_epoch, exit.clone()))
}

fn exited_plan(runtime: &crate::model::RuntimeView) -> SessionPlan {
    match runtime.exit.as_ref() {
        Some(exit)
            if exit.runtime_epoch == runtime.runtime_epoch
                && runtime.last_output_sequence == exit.last_output_sequence
                && exit.reason != ExitReason::Natural =>
        {
            if exit.reason == ExitReason::UserTerminate {
                SessionPlan::Attach(PreparedSession::UserTerminated)
            } else {
                SessionPlan::Resume
            }
        }
        _ => SessionPlan::PendingFinalization,
    }
}

fn active_plan() -> SessionPlan {
    SessionPlan::Attach(PreparedSession::Active)
}

fn is_resume_race(error: &ClientError) -> bool {
    matches!(error, ClientError::Remote { code, .. } if code == HOST_ALREADY_RUNNING_CODE)
}
