use crate::State;
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_rpc_protocol::Command;

pub fn not_sent(state: &mut State, request: ControlRequest) {
    if crate::control::probe_completed(state, &request, None)
        || crate::chat::not_sent(state, &request)
    {
        return;
    }
    crate::control::not_sent(state, &request);
    if matches!(request.command, Command::Input { .. }) {
        if crate::terminal_input_events::finished(state, &request, None).is_none() {
            let _ = crate::query_replies::completed(state, &request, None);
        }
        return;
    }
    if crate::initial_control::not_sent(state, &request) {
        return;
    }
    if matches!(request.command, Command::QueryCreation { .. }) {
        state
            .creations
            .recovery
            .not_sent(&request, std::time::Instant::now());
        return;
    }
    if matches!(request.command, Command::Finalize { .. }) {
        crate::finalization_events::not_sent(state, &request, std::time::Instant::now());
        return;
    }
    if crate::creation_completion::matches(state, &request) {
        let Command::Create { request: creation } = &request.command else {
            return;
        };
        let present = state
            .creations
            .pending
            .get(&creation.fields().request_id)
            .is_some_and(|pending| crate::creation_presentation::current(state, pending));
        crate::creation_rejection::retire(state, creation);
        if present {
            crate::creation_rejection::present(state, crate::constants::QUEUE_FULL_LABEL.into());
        }
        return;
    }
    if matches!(request.command, Command::Create { .. }) {
        return;
    }
    if crate::respond_events::not_sent(state, &request) {
        return;
    }
    if crate::interrupt_events::not_sent(state, &request) {
        return;
    }
    if request.generation != state.control_generation {
        return;
    }
    state.status = crate::constants::QUEUE_FULL_LABEL.into();
    let grant = match request.command {
        Command::Resize { request } => {
            crate::resize::failed(state, &request);
            return;
        }
        Command::Renew { grant } => grant,
        _ => return,
    };
    let address = grant.fields().target.address;
    if state
        .writers
        .get(&address)
        .is_some_and(|writer| writer.grant == grant)
    {
        state.clear_renewal_pending(address);
    }
}
