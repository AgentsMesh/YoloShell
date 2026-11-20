use crate::{
    Options,
    constants::QUEUE_CAPACITY,
    network::{Job, Update},
};
use tokio::sync::{mpsc, watch};
use yolo_hub_client_channels::{ControlRequest, Endpoint};
use yolo_hub_rpc_protocol::Command;
macro_rules! drop_channels { ($($channel:ident),+) => { $(drop($channel);)+ }; }
pub async fn network(options: Options, jobs: mpsc::Receiver<Job>, updates: mpsc::Sender<Update>) {
    network_with_priority(options, jobs, updates.clone(), updates, false).await;
}
pub async fn network_with_priority(
    options: Options,
    mut jobs: mpsc::Receiver<Job>,
    updates: mpsc::Sender<Update>,
    priority_updates: mpsc::Sender<Update>,
    subscribe_events: bool,
) {
    let (commands, requests) = mpsc::channel(QUEUE_CAPACITY);
    let (maintenance, renewals) = mpsc::channel(QUEUE_CAPACITY);
    let (inputs, input_requests) = mpsc::channel(QUEUE_CAPACITY);
    let input_owner =
        crate::input_dispatch::run(input_requests, commands.clone(), priority_updates.clone());
    let (refresh, refreshes) = watch::channel(None);
    let (control_events, controls) = mpsc::channel(QUEUE_CAPACITY);
    let (catalog_events, catalogs) = mpsc::channel(QUEUE_CAPACITY);
    let endpoint = Endpoint {
        entry: options.endpoint.clone(),
        hub: options.hub,
    };
    let (management, management_requests) =
        mpsc::channel(crate::management_network::QUEUE_CAPACITY);
    let (layouts, layout_requests) = mpsc::channel(crate::layout_sync::QUEUE_CAPACITY);
    let (chat, chat_requests) = mpsc::channel(crate::chat::network::CAPACITY);
    let (deferred_updates, update_forwarder) = crate::update_queue::Queue::new();
    let dispatch = async {
        while let Some(job) = jobs.recv().await {
            match job {
                Job::Chat(operation) => {
                    if let Err(error) = chat.try_send(operation) {
                        deferred_updates.normal(Update::Chat {
                            operation: error.into_inner(),
                            result: Err(yolo_tui_chat::QueryError::Disconnected),
                        });
                    }
                }
                Job::Layout(operation) => {
                    if let Err(error) = layouts.try_send(operation) {
                        deferred_updates.normal(Update::Layout {
                            operation: error.into_inner(),
                            result: Err(crate::constants::QUEUE_FULL_LABEL.into()),
                        });
                    }
                }
                Job::UserInput(request) => {
                    if let Err(error) = inputs.try_send(request) {
                        deferred_updates.priority(Update::NotSent {
                            request: error.into_inner(),
                        });
                    }
                }
                Job::CreationQuery(request) | Job::Scoped(request) => {
                    if let Some(request) = crate::input_dispatch::enqueue(&commands, request) {
                        deferred_updates.priority(Update::NotSent { request });
                    }
                }
                Job::Finalize {
                    generation,
                    creation_token,
                    creation,
                    validity,
                } => {
                    if let Err(error) = commands.try_send(ControlRequest {
                        generation: Some(generation),
                        destination: creation.address.hub_id,
                        command: yolo_hub_rpc_protocol::Command::Finalize {
                            creation_token,
                            creation,
                        },
                        validity: Some(validity),
                    }) {
                        deferred_updates.priority(Update::NotSent {
                            request: error.into_inner(),
                        });
                    }
                }
                Job::TerminalReply {
                    generation,
                    request,
                    validity,
                } => {
                    if let Err(error) = commands.try_send(ControlRequest {
                        generation: Some(generation),
                        destination: request.fields().grant.fields().target.address.hub_id,
                        command: yolo_hub_rpc_protocol::Command::Input { request },
                        validity: Some(validity),
                    }) {
                        deferred_updates.priority(Update::NotSent {
                            request: error.into_inner(),
                        });
                    }
                }
                Job::Management(operation) => {
                    if let Err(error) = management.try_send(operation) {
                        deferred_updates.priority(Update::ManagementNotSent {
                            token: error.into_inner().token,
                        });
                    }
                }
                Job::Refresh { request_id } => {
                    refresh.send_replace(Some(request_id));
                }
                Job::InitialAcquire {
                    generation,
                    origin,
                    request,
                    validity,
                }
                | Job::Acquire {
                    generation,
                    origin,
                    request,
                    validity,
                } => {
                    let control = ControlRequest {
                        validity: Some(validity),
                        generation: Some(generation),
                        destination: origin,
                        command: Command::Lease { request },
                    };
                    if let Err(error) = commands.try_send(control) {
                        deferred_updates.priority(Update::NotSent {
                            request: error.into_inner(),
                        });
                    }
                }
                Job::Call {
                    generation,
                    origin,
                    command,
                } => {
                    let destination = if matches!(command, Command::Renew { .. }) {
                        &maintenance
                    } else {
                        &commands
                    };
                    if let Some(request) = crate::input_dispatch::enqueue(
                        destination,
                        ControlRequest {
                            validity: None,
                            generation,
                            destination: origin,
                            command,
                        },
                    ) {
                        deferred_updates.priority(Update::NotSent { request });
                    }
                }
            }
        }
        #[rustfmt::skip]
        drop_channels!(commands, maintenance, inputs, refresh, management, layouts, chat);
        drop(deferred_updates);
    };
    let forward_updates = update_forwarder.run(updates.clone(), priority_updates.clone());
    let control_updates = crate::forwarding::controls(controls, priority_updates.clone());
    let catalog_updates = crate::forwarding::catalogs(catalogs, updates.clone());
    let platform_updates =
        crate::forwarding::platforms(endpoint.clone(), updates.clone(), subscribe_events);
    tokio::join!(
        dispatch,
        forward_updates,
        input_owner,
        yolo_hub_client_channels::run(
            endpoint,
            requests,
            renewals,
            refreshes,
            control_events,
            catalog_events
        ),
        control_updates,
        catalog_updates,
        platform_updates,
        crate::management_network::run(&options, management_requests, &updates),
        crate::layout_sync::network::run(&options, layout_requests, &updates),
        crate::chat::network::run(&options, chat_requests, &updates),
    );
}
