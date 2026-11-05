use super::fixtures;
use yolo_hub_terminal_protocol::TerminalSummary;

#[test]
fn terminal_selectors_accept_label_id_address_and_short_form() {
    let summary = TerminalSummary::new(fixtures::summary_fields()).unwrap();
    let fields = summary.fields();
    for selector in [
        fields.name.clone(),
        fields.address.terminal_id.to_string(),
        fields.address.to_string(),
        format!(
            "{}/{}",
            fields.address.hub_id, fields.address.terminal_id
        ),
    ] {
        assert!(summary.matches_selector(&selector), "{selector}");
    }
    for selector in ["", "other", &fields.name.to_uppercase()] {
        assert!(!summary.matches_selector(selector), "{selector}");
    }
}

#[test]
fn archived_summaries_still_match_for_callers_that_need_history() {
    let mut fields = fixtures::summary_fields();
    fields.archived = true;
    let summary = TerminalSummary::new(fields).unwrap();
    assert!(summary.matches_selector(&summary.fields().name));
}
