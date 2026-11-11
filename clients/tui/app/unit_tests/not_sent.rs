use super::fixtures;
use crate::{channel_events::not_sent, resize::PendingResize};
use uuid::Uuid;
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{ResizeRequest, TerminalSize};

#[test]
fn queue_rejection_preserves_authority_and_retires_only_its_exact_pending_work() {
    let session = fixtures::session();
    let grant = fixtures::grant(&session);
    let other = fixtures::grant(&session);
    let address = session.fields().address;
    let generation = Some(Uuid::new_v4());
    let mut state = crate::client_capability_support::state();
    state.control_generation = generation;
    state.record_grant(grant, std::time::Instant::now());
    let resize = |grant| ResizeRequest {
        request_id: Uuid::new_v4(),
        grant,
        size: TerminalSize::default(),
    };
    for (command, retires_resize, retires_renew) in [
        (
            Command::Resize {
                request: resize(other),
            },
            false,
            false,
        ),
        (Command::Renew { grant: other }, false, false),
        (Command::Hello {}, false, false),
        (
            Command::Resize {
                request: resize(grant),
            },
            false,
            false,
        ),
        (Command::Renew { grant }, false, true),
    ] {
        state.mark_resize_pending(address);
        state.mark_renewal_pending(address);
        state.set_last_size(address, TerminalSize::default());
        state.clear_all_resize_in_flight();
        if let Command::Resize { request } = &command {
            state.set_resize_in_flight(
                address,
                PendingResize {
                    request_id: request.request_id,
                    grant: request.grant,
                    size: request.size,
                },
            );
        }
        not_sent(
            &mut state,
            ControlRequest {
                validity: None,
                generation,
                destination: address.hub_id,
                command,
            },
        );
        assert_eq!(state.is_resize_pending(address), !retires_resize);
        assert_eq!(state.last_size(address).is_some(), !retires_resize);
        assert_eq!(state.renewal_pending(address), !retires_renew);
        assert_eq!(state.writers[&address].grant, grant);
        assert_eq!(state.control_generation, generation);
    }
    state.status = crate::constants::CONNECTED_LABEL.into();
    not_sent(
        &mut state,
        ControlRequest {
            validity: None,
            generation: Some(Uuid::new_v4()),
            destination: address.hub_id,
            command: Command::Resize {
                request: resize(grant),
            },
        },
    );
    assert_eq!(state.status, crate::constants::CONNECTED_LABEL);
}
