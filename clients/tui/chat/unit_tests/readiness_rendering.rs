use super::fixture::*;
#[path = "../test_support/render.rs"]
mod support;
use yolo_tui_chat::InputReadiness;

const SEND_HINT: &str = "Enter append & send";
const APPEND_HINT: &str = "Append & send";
const WAITING_HINT: &str = "Waiting for current agent state";
const UNAVAILABLE_HINT: &str = "Agent cannot accept a prompt";

#[test]
fn loaded_history_without_a_submission_does_not_offer_unavailable_input_in_its_footer() {
    let (mut chat, _) = ready();
    let transcript = page(&chat, &[1]);
    load(&mut chat, transcript);
    assert!(chat.composer.phase().is_none());
    for (input, expected) in [
        (InputReadiness::AwaitingObservation, WAITING_HINT),
        (InputReadiness::Unavailable, UNAVAILABLE_HINT),
    ] {
        let text = support::render_with_input(&chat, WIDTH, HEIGHT, input).0;
        assert!(text.contains(expected));
        assert!(!text.contains(SEND_HINT));
        assert!(!text.contains(APPEND_HINT));
    }
    let text = support::render_with_input(&chat, WIDTH, HEIGHT, InputReadiness::Ready).0;
    assert!(text.contains(SEND_HINT));
    assert!(text.contains(APPEND_HINT));
}
