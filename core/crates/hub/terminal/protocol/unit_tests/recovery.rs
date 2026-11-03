use super::fixtures::*;
use yolo_hub_terminal_protocol::{TerminalRecoveryHint, TerminalSummary, field};

#[test]
fn absent_hints_and_capabilities_remain_unknown_on_the_wire() {
    let value = TerminalSummary::new(summary_fields()).unwrap();
    let encoded = serde_json::to_value(&value).unwrap();
    assert!(encoded.get(field::RECOVERY).is_none());
    assert!(
        encoded[field::RUNTIME]
            .get(field::SAFE_TERMINATION)
            .is_none()
    );
    roundtrip(&value);
}

#[test]
fn unreachable_views_remove_stale_mutation_affordances_without_rewriting_identity() {
    let mut fields = summary_fields();
    fields.recovery = Some(TerminalRecoveryHint::Resume);
    fields.runtime.as_mut().unwrap().safe_termination = Some(true);
    let address = fields.address;
    let mut summary = TerminalSummary::new(fields).unwrap();
    roundtrip(&summary);
    summary.mark_unreachable();
    assert_eq!(summary.fields().address, address);
    assert_eq!(
        summary.fields().recovery,
        Some(TerminalRecoveryHint::Unavailable)
    );
    assert_eq!(summary.fields().runtime.unwrap().safe_termination, None);
    roundtrip(&summary);
}
