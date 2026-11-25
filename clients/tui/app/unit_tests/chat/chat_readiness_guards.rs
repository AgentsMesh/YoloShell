use super::{chat_support::*, drawing_support};
use crate::chat;
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::LeaseGrant;
use yolo_tui_chat::InputReadiness;

const SEND_HINT: &str = "Enter append & send";

#[derive(Clone, Copy, Debug)]
enum LostAuthority {
    Catalog,
    Agent,
    Runtime,
    Controller,
    Offline,
    Exited,
}

#[test]
fn readiness_and_manual_dispatch_reject_lost_authority_without_losing_the_draft() {
    for loss in [
        LostAuthority::Catalog,
        LostAuthority::Agent,
        LostAuthority::Runtime,
        LostAuthority::Controller,
        LostAuthority::Offline,
        LostAuthority::Exited,
    ] {
        let (mut state, jobs, mut incoming) = ready();
        toggle(&mut state, &jobs);
        paste(&mut state, &jobs, PROMPT);
        let target = chat::agent(&state).unwrap().fields().terminal;
        let mut grant = *state.writers[&target.address].grant.fields();
        let expected = match loss {
            LostAuthority::Catalog => {
                state.inventory.current = false;
                InputReadiness::AwaitingObservation
            }
            LostAuthority::Agent => {
                state.inventory.agents.clear();
                InputReadiness::Unavailable
            }
            LostAuthority::Runtime => {
                grant.target.runtime_epoch = RuntimeEpoch::new();
                state.writers.get_mut(&target.address).unwrap().grant =
                    LeaseGrant::new(grant).unwrap();
                InputReadiness::Unavailable
            }
            LostAuthority::Controller => {
                grant.principal.controller_instance_id = uuid::Uuid::new_v4();
                state.writers.get_mut(&target.address).unwrap().grant =
                    LeaseGrant::new(grant).unwrap();
                InputReadiness::Unavailable
            }
            LostAuthority::Offline => {
                state.mark_offline(target.address);
                InputReadiness::Unavailable
            }
            LostAuthority::Exited => {
                state.mark_ended(target.address, target.runtime_epoch);
                InputReadiness::Unavailable
            }
        };
        assert_eq!(chat::input_readiness(&state), expected, "{loss:?}");
        assert!(
            !drawing_support::render(&state, WIDTH, HEIGHT)
                .0
                .contains(SEND_HINT),
            "{loss:?}"
        );
        chat::submit_for_test(&mut state, &jobs);
        assert!(incoming.try_recv().is_err(), "{loss:?}");
        assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
        assert!(chat::current(&state).unwrap().composer.request().is_none());
    }
}
