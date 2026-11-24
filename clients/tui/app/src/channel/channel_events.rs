use crate::{
    State, catalog_events,
    constants::CONNECTED_LABEL,
    events,
    network::{Job, Update},
};
use tokio::sync::mpsc;
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_rpc_protocol::Command;

#[path = "channel_events/unsent.rs"]
mod unsent;
pub use unsent::not_sent;
pub fn apply(state: &mut State, jobs: &mpsc::Sender<Job>, event: ControlEvent) {
    match event {
        ControlEvent::Ready { generation, hello } => {
            if generation.is_nil() || state.control_generation == Some(generation) {
                return;
            }
            state.disconnected();
            if !crate::client_capabilities::install(state, &hello) {
                return;
            }
            state.recoverable_status = None;
            state.control_generation = Some(generation);
            state.workspaces_supported = hello
                .methods
                .contains(&yolo_hub_rpc_protocol::Method::ListWorkspaces);
            // An explicit attach is still searching catalogue pages for its
            // target. Channel readiness must not replace that visible intent
            // with the routine connected label.
            if state.status != crate::startup_constants::SEARCHING_TERMINAL {
                state.status = CONNECTED_LABEL.into();
            }
            catalog_events::invalidate(state, jobs);
            crate::finalization_actions::flush(state, jobs, std::time::Instant::now());
            crate::creation_recovery::flush(state, jobs, std::time::Instant::now());
        }
        ControlEvent::Unreachable {
            generation,
            message,
        } => {
            if state.control_generation == generation {
                state.disconnected();
                state.status = message;
            }
        }
        ControlEvent::Finished {
            request,
            result,
            connected,
        } => {
            if matches!(request.command, Command::QueryCreation { .. }) {
                let now = std::time::Instant::now();
                if state.creations.recovery.finished(&request, result, now) {
                    crate::creation_recovery_persistence::flush(state, now);
                    crate::finalization_actions::flush(state, jobs, now);
                    catalog_events::invalidate(state, jobs);
                }
                return;
            }
            if matches!(request.command, Command::Finalize { .. }) {
                let matched = crate::finalization_events::finished(
                    state,
                    &request,
                    result,
                    std::time::Instant::now(),
                );
                if request.generation == state.control_generation && !connected {
                    state.disconnected();
                }
                if matched {
                    catalog_events::invalidate(state, jobs);
                }
                return;
            }
            if matches!(request.command, Command::Create { .. }) {
                crate::creation_completion::apply(state, jobs, &request, result);
                if request.generation == state.control_generation && !connected {
                    state.disconnected();
                }
                catalog_events::invalidate(state, jobs);
                return;
            }
            if crate::terminal_input_events::control(state, &request, &result, connected) {
                return;
            }
            if crate::chat::control(state, &request, &result, connected) {
                return;
            }
            if crate::control::probe_completed(state, &request, Some(&result)) {
                if request.generation == state.control_generation && !connected {
                    state.disconnected();
                }
                return;
            }
            if let Some(matched) = crate::respond_events::finished(state, &request, &result) {
                if matched && request.generation == state.control_generation && !connected {
                    state.disconnected();
                }
                return;
            }
            if let Some(matched) = crate::interrupt_events::finished(state, &request, &result) {
                if matched && request.generation == state.control_generation && !connected {
                    state.disconnected();
                }
                return;
            }
            if request.generation != state.control_generation
                || !crate::control::observe_reply(state, &request)
            {
                return;
            }
            if crate::terminal_feedback::obsolete(state, &request.command) {
                if !connected {
                    state.disconnected();
                }
                return;
            }
            let barrier = catalog_events::changes(&request.command);
            let update = match result {
                Ok(reply) => Update::Reply {
                    command: request.command,
                    reply,
                },
                Err(error) => {
                    crate::control::failed(state, &request, &error);
                    let message =
                        crate::terminal_feedback::message(state, &request.command, &error);
                    let transient = connected
                        && matches!(
                            &error,
                            yolo_hub_client_channels::ControlError::Client(
                                yolo_hub_client::ClientError::Transport(_)
                            )
                        );
                    Update::Failed {
                        command: Some(request.command),
                        message,
                        connected,
                        transient,
                    }
                }
            };
            events::apply(state, jobs, update);
            if !connected {
                state.disconnected();
            }
            if barrier {
                catalog_events::invalidate(state, jobs);
            }
        }
    }
}
