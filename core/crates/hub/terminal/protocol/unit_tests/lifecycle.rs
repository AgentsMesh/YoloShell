use crate::fixtures::{self, constants::*, roundtrip};
use uuid::Uuid;
use yolo_hub_identity::{RuntimeEpoch, TerminalId};
use yolo_hub_terminal_protocol::*;

#[test]
fn no_runtime_selection_is_an_exact_precondition_not_a_wildcard() {
    let running = TerminalSummary::new(fixtures::summary_fields()).unwrap();
    let mut selection = TerminalSelection {
        address: fixtures::address(),
        expected_runtime: None,
    };
    assert_eq!(
        selection.validate_against(&running),
        Err(PublicError::TargetMismatch)
    );
    selection.expected_runtime = Some(fixtures::target().runtime_epoch);
    assert_eq!(selection.validate_against(&running), Ok(()));
    selection.expected_runtime = Some(RuntimeEpoch::new());
    assert_eq!(
        selection.validate_against(&running),
        Err(PublicError::TargetMismatch)
    );
    selection.expected_runtime = Some(fixtures::target().runtime_epoch);
    selection.address.terminal_id = TerminalId::new();
    assert_eq!(
        selection.validate_against(&running),
        Err(PublicError::TargetMismatch)
    );
    let mut fields = fixtures::summary_fields();
    fields.runtime = None;
    let selection = TerminalSelection {
        address: fixtures::address(),
        expected_runtime: None,
    };
    assert_eq!(
        selection.validate_against(&TerminalSummary::new(fields).unwrap()),
        Ok(())
    );
    roundtrip(&selection);
}

#[test]
fn lifecycle_actions_require_non_nil_request_identity() {
    for action in [
        TerminalAction::Resume {},
        TerminalAction::Recreate {},
        TerminalAction::Terminate { force: false },
        TerminalAction::Terminate { force: true },
        TerminalAction::Archive {},
    ] {
        let mut request = TerminalCommand {
            request_id: Uuid::from_u128(REQUEST_ID),
            selection: TerminalSelection {
                address: fixtures::address(),
                expected_runtime: Some(fixtures::target().runtime_epoch),
            },
            action,
        };
        assert_eq!(request.validate(), Ok(()));
        roundtrip(&request);
        request.request_id = Uuid::nil();
        assert_eq!(request.validate(), Err(PublicError::InvalidRequest));
    }
}
