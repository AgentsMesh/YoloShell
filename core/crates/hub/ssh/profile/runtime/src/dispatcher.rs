use crate::SshProfileRuntime;
use std::sync::Arc;
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply, Request, SSH_PROFILE_METHODS};
use yolo_hub_terminal_protocol::PublicError;

pub(crate) struct ProfileDispatcher {
    pub profiles: Arc<SshProfileRuntime>,
    pub inner: Arc<dyn Dispatcher>,
}

impl Dispatcher for ProfileDispatcher {
    fn hub_id(&self) -> HubId {
        self.profiles.state.hub_id()
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            self.profiles.state.verify().map_err(|_| {
                self.profiles.close_all();
                PublicError::Unreachable
            })?;
            let local = request.fields().destination == self.hub_id();
            match &request.fields().command {
                Command::GetSshProfiles {}
                | Command::ConfigureSshProfiles { .. }
                | Command::ConnectSshProfile { .. }
                    if !local =>
                {
                    return Err(PublicError::Unauthorized);
                }
                Command::ConnectSshProfile {
                    profile,
                    credentials,
                } => {
                    let deadline =
                        tokio::time::Instant::now() + yolo_remote_broker::BROKER_READY_TIMEOUT;
                    let binding = self
                        .profiles
                        .connect_profile(
                            profile,
                            credentials.clone().map(|value| value.into_bytes()),
                            deadline,
                        )
                        .await
                        .map_err(|error| {
                            tracing::warn!(
                                profile = profile.profile_id(),
                                error = %format_args!("{error:#}"),
                                message = crate::constants::CONNECT_FAILED,
                            );
                            PublicError::OutcomeUnknown
                        })?;
                    return Ok(Operation::Reply(Reply::SshConnected { binding }));
                }
                Command::GetSshProfiles {} => {
                    let store = self.profiles.store.lock().map_err(|_| {
                        self.profiles.close_all();
                        PublicError::Unreachable
                    })?;
                    let state = store.current().map_err(|_| {
                        self.profiles.close_all();
                        PublicError::Unreachable
                    })?;
                    self.profiles.state.verify().map_err(|_| {
                        self.profiles.close_all();
                        PublicError::Unreachable
                    })?;
                    return Ok(Operation::Reply(Reply::SshProfiles { state }));
                }
                Command::ConfigureSshProfiles { change } => {
                    change.validate().map_err(|_| PublicError::InvalidRequest)?;
                    let mut store = self.profiles.store.lock().map_err(|_| {
                        self.profiles.close_all();
                        PublicError::Unreachable
                    })?;
                    if !self
                        .profiles
                        .reservations
                        .lock()
                        .map_err(|_| PublicError::Unreachable)?
                        .permits(&change.configuration)
                    {
                        return Err(PublicError::Conflict);
                    }
                    let state = match store.apply(change.clone()) {
                        Ok(state) => state,
                        Err(error) if error.is::<yolo_hub_ssh_profiles::RevisionConflict>() => {
                            return Err(PublicError::Conflict);
                        }
                        Err(_) => {
                            self.profiles.close_all();
                            return Err(PublicError::OutcomeUnknown);
                        }
                    };
                    if self
                        .profiles
                        .revoke_changed(&state.configuration)
                        .and_then(|()| store.verify())
                        .is_err()
                    {
                        self.profiles.close_all();
                        return Err(PublicError::OutcomeUnknown);
                    }
                    self.profiles.changed.send_replace(state.revision);
                    self.profiles.state.verify().map_err(|_| {
                        self.profiles.close_all();
                        PublicError::OutcomeUnknown
                    })?;
                    return Ok(Operation::Reply(Reply::SshProfiles { state }));
                }
                _ => {}
            }
            let hello = local && matches!(request.fields().command, Command::Hello {});
            let mut outcome = self.inner.dispatch(request).await?;
            if hello {
                if let Operation::Reply(Reply::Hello { hello }) = &mut outcome {
                    for method in SSH_PROFILE_METHODS {
                        if !hello.methods.contains(method) {
                            hello.methods.push(*method);
                        }
                    }
                }
            }
            Ok(outcome)
        })
    }
}
