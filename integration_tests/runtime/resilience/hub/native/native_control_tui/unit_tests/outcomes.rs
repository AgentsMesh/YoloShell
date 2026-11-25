use super::{
    constants::*,
    outcome::{self, Outcome},
};

const IDENTITY: &str = "Agent expected-agent";
const OTHER_IDENTITY: &str = "Agent replaced-agent";
const WRITING: &str = "target | Control: this UI";
const READ_ONLY: &str = "target | Read Only · Ctrl-b t to take control";
const CONNECTED: &str = "Agent runs: 1 | Connected";
const REFRESHING: &str = "Agent runs: 1 | Refreshing";
const REFRESHING_BLOCKED: &str = "Agent runs: 1 | Refreshing | Shell confirmations blocked: 1";
const STALE_AGENT: &str = "Unknown | Stale";
const FILTER: &str = "Filter: target";

fn screen(header: &str, agent: &str, writer: &str, freshness: &str, footer: &str) -> String {
    format!("{header}\n{agent}\n{writer}\n{freshness}\n{footer}")
}

#[test]
fn unconfirmed_or_uncertain_receipts_never_authorize_another_user_request() {
    for (status, expected) in [
        (SENDING, Outcome::Pending),
        (ACCEPTED, Outcome::Pending),
        (UNKNOWN, Outcome::Unknown),
    ] {
        let visible = screen(CONNECTED, IDENTITY, WRITING, CURRENT_AGENT, status);
        assert!(outcome::ready(&visible, IDENTITY, WRITING));
        assert_eq!(outcome::outcome(&visible), expected);
    }
}

#[test]
fn only_the_complete_current_footer_classifies_a_known_refusal() {
    assert_eq!(outcome::outcome(STALE), Outcome::PreconditionsChanged);
    assert_eq!(
        outcome::outcome(&format!("{STALE}{INTERRUPT_HINT}")),
        Outcome::NotSent
    );
    assert_eq!(outcome::outcome(NOT_SENT), Outcome::NotSent);
    assert_eq!(
        outcome::outcome(&format!("{UNKNOWN}{INTERRUPT_HINT}")),
        Outcome::Blocked
    );
    assert_eq!(
        outcome::outcome(&format!("{SENT}{INTERRUPT_HINT}")),
        Outcome::Blocked
    );
}

#[test]
fn terminal_text_and_a_cached_rejection_cannot_override_the_new_sending_receipt() {
    let visible = format!("{CONNECTED}\n{STALE}\n{UNKNOWN}\n{NOT_SENT}\n{SENT}\n{SENDING}");
    assert_eq!(outcome::outcome(&visible), Outcome::Pending);
}

#[test]
fn selected_agent_title_prefix_preserves_exact_footer_receipt_classification() {
    for (receipt, expected) in [
        (SENT, Outcome::Sent),
        (SENDING, Outcome::Pending),
        (UNKNOWN, Outcome::Unknown),
    ] {
        let visible = format!("{CONNECTED}\n{WRITING}\n{IDENTITY}{STATUS_SEPARATOR}{receipt}");
        assert_eq!(outcome::outcome(&visible), expected);
    }
    let retry = format!("{IDENTITY}{STATUS_SEPARATOR}{STALE}{INTERRUPT_HINT}");
    assert_eq!(outcome::outcome(&retry), Outcome::NotSent);
}

#[test]
fn search_and_refresh_phases_cannot_pass_the_inventory_ready_barrier() {
    let before = screen(CONNECTED, IDENTITY, WRITING, CURRENT_AGENT, STALE);
    assert!(outcome::ready(&before, IDENTITY, WRITING));
    assert!(!outcome::searching(&before));
    let searching = screen(CONNECTED, IDENTITY, WRITING, CURRENT_AGENT, FILTER);
    assert!(outcome::searching(&searching));
    assert!(!outcome::ready(&searching, IDENTITY, WRITING));
    let refreshing = screen(REFRESHING, IDENTITY, WRITING, CURRENT_AGENT, STALE);
    assert!(!outcome::searching(&refreshing));
    assert!(!outcome::ready(&refreshing, IDENTITY, WRITING));
    let blocked = screen(REFRESHING_BLOCKED, IDENTITY, WRITING, CURRENT_AGENT, STALE);
    assert!(!outcome::ready(&blocked, IDENTITY, WRITING));
    let after = screen(CONNECTED, IDENTITY, WRITING, CURRENT_AGENT, NOT_SENT);
    assert!(!outcome::searching(&after));
    assert!(outcome::ready(&after, IDENTITY, WRITING));
}

#[test]
fn refresh_completion_requires_the_original_agent_current_observation_and_writer() {
    for (agent, writer, freshness) in [
        (OTHER_IDENTITY, WRITING, CURRENT_AGENT),
        (IDENTITY, READ_ONLY, CURRENT_AGENT),
        (IDENTITY, WRITING, STALE_AGENT),
    ] {
        let visible = screen(CONNECTED, agent, writer, freshness, STALE);
        assert!(!outcome::ready(&visible, IDENTITY, WRITING));
    }
}
