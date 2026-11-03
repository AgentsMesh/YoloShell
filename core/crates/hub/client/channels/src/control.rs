use crate::{
    ControlError, ControlEvent, Endpoint, admission::admit, constants::*, input_batch::Dispatch,
};
use tokio::{
    sync::{mpsc, watch},
    time::{MissedTickBehavior, interval},
};
use uuid::Uuid;
use yolo_hub_rpc_protocol::Command;

#[path = "control_connect.rs"]
mod control_connect;
#[path = "control_publish.rs"]
mod control_publish;
#[path = "control_writer.rs"]
mod control_writer;

pub(crate) async fn run(
    endpoint: Endpoint,
    mut commands: mpsc::Receiver<Dispatch>,
    updates: mpsc::Sender<ControlEvent>,
    generation_updates: watch::Sender<Option<Uuid>>,
) {
    let mut generation = None;
    let mut operations = crate::operations::Operations::default();
    let mut queries = crate::queries::Queries::default();
    let mut retry = interval(RECONNECT_DELAY);
    retry.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let (writer_sender, writer_input) = mpsc::unbounded_channel();
    let mut writer_commands = Some(writer_sender);
    let (writer_updates, mut writer_events) = mpsc::channel(INPUT_DISPATCH_CAPACITY);
    let writer_task = tokio::spawn(control_writer::run(
        writer_input,
        writer_updates,
        generation_updates.subscribe(),
    ));
    let mut connected = false;
    loop {
        tokio::select! {
            biased;
            event = writer_events.recv() => {
                let Some(event) = event else { break };
                let completed = match event {
                    control_writer::Event::Completed(completed) => completed,
                    control_writer::Event::Disconnected { generation: failed, message } => {
                        if generation != Some(failed) { continue; }
                        let previous = generation.take();
                        connected = false;
                        generation_updates.send_replace(None);
                        retry.reset();
                        let _ = updates.send(ControlEvent::Unreachable {
                            generation: previous, message,
                        }).await;
                        continue;
                    }
                };
                let acknowledged = completed.acknowledged;
                let still_connected = connected && generation.is_some() && completed.connected;
                if !completed.connected && generation == completed.request.generation {
                    let previous = generation;
                    generation.take();
                    connected = false;
                    generation_updates.send_replace(None);
                    retry.reset();
                    let _ = writer_commands
                        .as_ref()
                        .and_then(|sender| sender.send(control_writer::Command::Reset).ok());
                    if previous.is_some() {
                        let _ = updates
                            .send(ControlEvent::Unreachable {
                                generation: previous,
                                message: NOT_CONNECTED.into(),
                            })
                            .await;
                    }
                }
                control_publish::input_results(
                    &updates,
                    completed.originals,
                    completed.result,
                    completed.dispatch,
                    still_connected,
                ).await;
                let _ = acknowledged.send(());
            }
            completed = operations.next(), if !operations.is_empty() && connected => {
                match completed {
                    Some(Ok(completed)) => {
                        let _ = updates.send(ControlEvent::Finished {
                            request: completed.request,
                            result: completed.result,
                            connected,
                        }).await;
                    }
                    Some(Err(_)) => {
                        let previous = generation.take();
                        generation_updates.send_replace(None);
                        connected = false;
                        retry.reset();
                        let _ = updates.send(ControlEvent::Unreachable {
                            generation: previous, message: OPERATION_TASK_FAILED.into(),
                        }).await;
                    }
                    None => {}
                }
            }
            completed = queries.next(), if !queries.is_empty() => {
                if let Some(completed) = completed {
                    control_connect::publish_query(&updates, completed, connected).await;
                }
            }
            command = commands.recv(), if writer_commands.is_some() => {
                let Some(command) = command else {
                    writer_commands.take();
                    continue;
                };
                let (request, originals) = match command {
                    Dispatch::Single(request) => (request.clone(), vec![request]),
                    Dispatch::Inputs { combined, originals } => (combined, originals),
                };
                let was_connected = connected;
                let admission = admit(generation, request.generation).and_then(|()| {
                    if request.validity.as_ref().is_some_and(|validity| !validity.is_current()) {
                        Err(ControlError::Invalidated)
                    } else { Ok(()) }
                });
                if admission.is_ok() && matches!(request.command, Command::QueryCreation { .. }) {
                    if let Err(error) = queries.start(&endpoint, &request, generation_updates.subscribe()) {
                        let _ = updates.send(ControlEvent::Finished {
                            request, result: Err(error), connected: was_connected,
                        }).await;
                    }
                    continue;
                }
                if admission.is_ok() && request.command.operation_timeout(std::time::Duration::ZERO) > std::time::Duration::ZERO {
                    if let Err(error) = operations.start(&endpoint, &request, generation_updates.subscribe()) {
                        let _ = updates.send(ControlEvent::Finished {
                            request, result: Err(error), connected: was_connected,
                        }).await;
                    }
                    continue;
                }
                if !connected {
                    let result = admission
                        .err()
                        .unwrap_or(ControlError::NotConnected);
                    control_publish::input_results(
                        &updates,
                        originals,
                        Err(result),
                        crate::input_completion::Dispatch::NotAttempted,
                        false,
                    ).await;
                } else {
                    let _ = writer_commands
                        .as_ref()
                        .and_then(|sender| sender.send(control_writer::Command::Call { request, originals }).ok());
                }
            }
            _ = retry.tick(), if !connected => {
                let Some(result) = control_connect::connect(&endpoint, &mut commands, &updates, &mut queries).await else { break };
                let event = match result {
                    Ok(client) => {
                        let fresh = Uuid::new_v4();
                        let hello = client.hello().clone();
                        let _ = writer_commands
                            .as_ref()
                            .and_then(|sender| sender.send(control_writer::Command::Attach {
                                generation: fresh, client,
                            }).ok());
                        connected = true;
                        generation = Some(fresh);
                        generation_updates.send_replace(generation);
                        ControlEvent::Ready {
                            generation: fresh,
                            hello,
                        }
                    }
                    Err(error) => {
                        retry.reset();
                        ControlEvent::Unreachable { generation: None, message: error.to_string() }
                    }
                };
                let _ = updates.send(event).await;
            }
        }
    }
    drop(writer_commands.take());
    let _ = writer_task.await;
    // No shutdown RPC is needed: dropping the private stream never stops a session.
}
