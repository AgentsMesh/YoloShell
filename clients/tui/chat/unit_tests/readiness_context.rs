use super::fixture::*;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::{ExecutionId, RuntimeEpoch};
use yolo_tui_chat::*;

#[derive(Clone, Copy, Debug)]
enum InvalidContext {
    TerminalView,
    RetiredExecution,
    DifferentExecution,
    DifferentRuntime,
    StaleObservation,
    UnreachableAgent,
}

#[test]
fn retained_draft_never_becomes_sendable_in_another_or_unobserved_execution() {
    for context in [
        InvalidContext::TerminalView,
        InvalidContext::RetiredExecution,
        InvalidContext::DifferentExecution,
        InvalidContext::DifferentRuntime,
        InvalidContext::StaleObservation,
        InvalidContext::UnreachableAgent,
    ] {
        let (mut chat, agent) = ready();
        chat.composer.insert(PROMPT).unwrap();
        let mut fields = agent.fields().clone();
        let expected_error = match context {
            InvalidContext::TerminalView => {
                chat.set_mode(Mode::Terminal);
                DraftError::WrongTarget
            }
            InvalidContext::RetiredExecution => {
                chat.retire();
                DraftError::WrongTarget
            }
            InvalidContext::DifferentExecution => {
                let mut target = fields.cursor.target();
                target.execution_id = ExecutionId::new();
                fields.cursor = ObservationCursor::new(
                    target,
                    fields.cursor.stream(),
                    fields.cursor.sequence(),
                )
                .unwrap();
                DraftError::WrongTarget
            }
            InvalidContext::DifferentRuntime => {
                fields.terminal.runtime_epoch = RuntimeEpoch::new();
                DraftError::WrongTarget
            }
            InvalidContext::StaleObservation => {
                fields.freshness = Freshness::Stale;
                DraftError::Unavailable
            }
            InvalidContext::UnreachableAgent => {
                fields.reachability = Reachability::Unreachable;
                DraftError::Unavailable
            }
        };
        if expected_error == DraftError::Unavailable {
            fields.capabilities.interrupt = Capability::Unavailable {
                reason: UnavailabilityReason::InputUnavailable,
            };
            fields.capabilities.submit_prompt = fields.capabilities.interrupt;
        }
        let observed = AgentSnapshot::new(fields).unwrap();
        assert_eq!(
            chat.input_readiness(&observed),
            InputReadiness::Unavailable,
            "{context:?}"
        );
        assert_eq!(chat.submit(&observed), Err(expected_error), "{context:?}");
        assert_eq!(chat.composer.text(), PROMPT);
        assert!(chat.composer.request().is_none());
    }
}
