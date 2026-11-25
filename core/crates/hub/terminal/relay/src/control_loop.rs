use crate::control::{Control, publish, request_deadline};
use crate::lanes::{InputTask, RenewalTask, ResizeTask, SemanticTask, wait};
use crate::{CommandChannel, Outcome, RelayError, Status, authority::retire_expired, constants::*};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::{mpsc, watch},
    time::{Instant, sleep_until},
};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{InputFields, InputRequest, TerminalSize};

impl Control {
    pub async fn run(
        &mut self,
        mut input: impl AsyncRead + Unpin,
        mut sizes: watch::Receiver<Option<TerminalSize>>,
        status: &mpsc::Sender<Status>,
        mut commands: Option<CommandChannel>,
    ) -> Result<Outcome, RelayError> {
        if retire_expired(&mut self.authority, Instant::now()) {
            self.initial = Status::ReadOnly { holder_id: None };
        }
        publish(status, self.initial.clone()).await?;
        let mut buffer = [0; INPUT_BUFFER_BYTES];
        let mut sizes_open = true;
        let mut input_task: Option<InputTask> = None;
        let mut renew_task: Option<RenewalTask> = None;
        let mut resize_task: Option<ResizeTask> = None;
        let mut semantic_task: Option<SemanticTask> = None;
        loop {
            let renewal = self
                .authority
                .as_ref()
                .map(|authority| authority.renew_at)
                .unwrap_or_else(|| Instant::now() + REQUEST_TIMEOUT);
            let controlled = self.authority.is_some();
            tokio::select! {
                biased;
                _ = sleep_until(renewal), if controlled && renew_task.is_none() => {
                    let authority = self.authority.as_ref().expect("controlled renewal");
                    let grant = authority.grant;
                    let sent = Instant::now();
                    if !authority.is_current(sent) {
                        self.authority = None;
                        self.lost(status).await?;
                        continue;
                    }
                    let deadline = request_deadline(authority);
                    let mut client = self.take_call_client().await?;
                    renew_task = Some(tokio::spawn(async move {
                        let result = client
                            .call(
                                grant.fields().target.address.hub_id,
                                Command::Renew { grant },
                                deadline,
                            )
                            .await;
                        (client, result, sent)
                    }));
                }
                renewed = wait(&mut renew_task), if renew_task.is_some() => {
                    let Some(renewed) = renewed else { continue };
                    let (client, result, sent) = renewed.map_err(|_| RelayError::Io)?;
                    self.return_call_client(client);
                    renew_task = None;
                    match self.accept(result)? {
                        Some(yolo_hub_rpc_protocol::Reply::Lease { grant }) => {
                            self.cleanup_grant = Some(grant);
                            self.authority = crate::authority::Authority::received(grant, sent, Instant::now());
                        }
                        None => {}
                        _ => return Err(RelayError::InvalidReply),
                    }
                    if self.authority.is_none() { self.lost(status).await?; }
                }
                completed = wait(&mut input_task), if input_task.is_some() => {
                    let Some(completed) = completed else { continue };
                    self.input_busy = false;
                    let (client, result) = completed.map_err(|_| RelayError::Io)?;
                    self.return_call_client(client);
                    input_task = None;
                    if self.accept(result)?.is_none() { self.lost(status).await?; }
                }
                command = crate::commands::next(&mut commands), if semantic_task.is_none() => {
                    let command = command.ok_or(RelayError::CommandClosed)?;
                    let launch = commands.as_ref().unwrap().launch_id;
                    if let Some(operation) = self.begin_semantic(command, launch, status).await? {
                        let mut client = self.take_call_client().await?;
                        let origin = self.origin();
                        semantic_task = Some(tokio::spawn(async move {
                            let result = client
                                .call(
                                    origin,
                                    Command::AgentControl {
                                        request: operation.public.clone(),
                                        grant: operation.grant,
                                    },
                                    operation.deadline,
                                )
                                .await;
                            (client, result, operation)
                        }));
                    }
                }
                semantic = wait(&mut semantic_task), if semantic_task.is_some() => {
                    let Some(semantic) = semantic else { continue };
                    let (client, result, operation) = semantic.map_err(|_| RelayError::Io)?;
                    let client_usable = client.is_usable();
                    self.return_call_client(client);
                    semantic_task = None;
                    self.finish_semantic(operation, client_usable, result, status).await?;
                }
                changed = sizes.changed(), if sizes_open => {
                    sizes_open = changed.is_ok();
                    if sizes_open {
                        let size = *sizes.borrow_and_update();
                        if let Some(size) = size {
                            self.options.size = Some(size);
                            if resize_task.is_none() {
                                resize_task = self.spawn_resize(size).await?;
                            }
                        }
                    }
                }
                resized = wait(&mut resize_task), if resize_task.is_some() => {
                    let Some(resized) = resized else { continue };
                    let (client, result, was_controlled, sent) =
                        resized.map_err(|_| RelayError::Io)?;
                    self.return_call_client(client);
                    resize_task = None;
                    self.accept(result)?;
                    if was_controlled && self.authority.is_none() { self.lost(status).await?; }
                    // A drag can deliver its final size while the previous
                    // RPC is in flight. The watch value was already marked
                    // seen, so completion must deliver the newer size itself.
                    if let Some(desired) = self.options.size.filter(|desired| *desired != sent) {
                        resize_task = self.spawn_resize(desired).await?;
                    }
                }
                length = input.read(&mut buffer), if input_task.is_none() => {
                    let length = length?;
                    if length == 0 { return Ok(Outcome::Detached); }
                    if retire_expired(&mut self.authority, Instant::now()) {
                        self.lost(status).await?;
                    }
                    if let Some(authority) = &self.authority {
                        let request = InputRequest::new(InputFields { request_id: uuid::Uuid::new_v4(), grant: authority.grant, data: buffer[..length].to_vec() }).map_err(|_| RelayError::InvalidOptions)?;
                        let origin = self.origin();
                        let deadline = request_deadline(authority);
                        // Renewal, resize and semantic calls may temporarily
                        // borrow the only connected client. Input has no
                        // reserved channel, so reconnect instead of treating a
                        // healthy Hub as a poisoned session.
                        let mut client = self.take_call_client().await?;
                        self.input_busy = true;
                        input_task = Some(tokio::spawn(async move {
                            let result = client
                                .call(
                                    origin,
                                    Command::Input { request },
                                    deadline,
                                )
                                .await;
                            (client, result)
                        }));
                    }
                }
            }
        }
    }
}
