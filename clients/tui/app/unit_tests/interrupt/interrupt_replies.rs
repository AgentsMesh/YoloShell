use super::interrupt_support::*;
use crate::{
    constants::QUEUE_CAPACITY, events, interrupt_actions::status, interrupt_constants as text,
    interrupt_state::Phase, network::Update,
};
use crossterm::event::KeyEventKind;
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::{CommandOutcome, CommandReceipt, FailureCode};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn delivered_labels_only_interrupt_sent_and_allows_a_later_explicit_new_request() {
    let mut state = ready();
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    press(&mut state, &jobs, KeyEventKind::Press);
    let first = queued(&mut incoming);
    finish(
        &mut state,
        &jobs,
        first.clone(),
        reply(&first, CommandOutcome::Delivered {}),
    );
    assert_eq!(status(&state), text::SENT);
    assert_eq!(
        state.interrupts.get(target(&state)).unwrap().phase,
        Phase::Sent
    );
    press(&mut state, &jobs, KeyEventKind::Repeat);
    assert!(incoming.try_recv().is_err());
    press(&mut state, &jobs, KeyEventKind::Press);
    let next = queued(&mut incoming);
    assert_ne!(
        request(&next).fields().request_id,
        request(&first).fields().request_id
    );
    finish(
        &mut state,
        &jobs,
        first.clone(),
        reply(&first, CommandOutcome::Unknown {}),
    );
    assert_eq!(
        state.interrupts.get(target(&state)).unwrap().request,
        *request(&next)
    );
    assert_eq!(status(&state), text::SENDING);
}

#[test]
fn unknown_and_accepted_results_block_duplicate_effects_without_automatic_replay() {
    for outcome in [CommandOutcome::Unknown {}, CommandOutcome::Accepted {}] {
        let mut state = ready();
        let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
        press(&mut state, &jobs, KeyEventKind::Press);
        let first = queued(&mut incoming);
        finish(&mut state, &jobs, first.clone(), reply(&first, outcome));
        for kind in [KeyEventKind::Press, KeyEventKind::Repeat] {
            press(&mut state, &jobs, kind);
        }
        assert!(incoming.try_recv().is_err());
        assert!(state.interrupts.get(target(&state)).unwrap().phase.blocks());
        assert_eq!(
            status(&state),
            if outcome == (CommandOutcome::Unknown {}) {
                text::UNKNOWN
            } else {
                text::ACCEPTED
            }
        );
    }
}

#[test]
fn wrong_receipt_id_target_or_reply_family_never_claim_delivery() {
    for invalid in [WrongReply::Request, WrongReply::Target, WrongReply::Family] {
        let mut state = ready();
        let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
        press(&mut state, &jobs, KeyEventKind::Press);
        let first = queued(&mut incoming);
        let fields = request(&first).fields();
        let mut target = fields.target;
        let id = if matches!(invalid, WrongReply::Request) {
            uuid::Uuid::new_v4()
        } else {
            fields.request_id
        };
        if matches!(invalid, WrongReply::Target) {
            target.execution_id = yolo_hub_identity::ExecutionId::new();
        }
        let mut receipt = CommandReceipt::accepted(id, target).unwrap();
        receipt.advance(CommandOutcome::Delivered {}).unwrap();
        let invalid = if matches!(invalid, WrongReply::Family) {
            Reply::Completed {}
        } else {
            Reply::AgentControl { receipt }
        };
        finish(&mut state, &jobs, first, invalid);
        assert_eq!(status(&state), text::UNKNOWN);
        press(&mut state, &jobs, KeyEventKind::Press);
        assert!(incoming.try_recv().is_err());
    }
}
#[derive(Clone, Copy)]
enum WrongReply {
    Request,
    Target,
    Family,
}

#[test]
fn definitive_failure_and_queue_notsent_allow_explicit_retry_but_ambiguous_error_does_not() {
    for error in [
        ControlError::NotConnected,
        ControlError::StaleGeneration,
        ControlError::Client(ClientError::Remote(PublicError::Unauthorized)),
        ControlError::Client(ClientError::OutcomeUnknownResponse(
            PublicError::InvalidRequest,
        )),
    ] {
        let mut state = ready();
        let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
        press(&mut state, &jobs, KeyEventKind::Press);
        let first = queued(&mut incoming);
        let unknown = matches!(
            error,
            ControlError::Client(ClientError::OutcomeUnknownResponse(_))
        );
        events::apply(
            &mut state,
            &jobs,
            Update::Control(ControlEvent::Finished {
                request: first,
                result: Err(error),
                connected: true,
            }),
        );
        press(&mut state, &jobs, KeyEventKind::Press);
        assert_eq!(incoming.try_recv().is_err(), unknown);
    }
    let mut state = ready();
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    press(&mut state, &jobs, KeyEventKind::Press);
    let first = queued(&mut incoming);
    finish(
        &mut state,
        &jobs,
        first.clone(),
        reply(
            &first,
            CommandOutcome::Failed {
                code: FailureCode::PreconditionsChanged,
            },
        ),
    );
    state.writers.clear();
    press(&mut state, &jobs, KeyEventKind::Press);
    assert!(status(&state).contains(text::TAKE_CONTROL));
    assert!(incoming.try_recv().is_err());
}
