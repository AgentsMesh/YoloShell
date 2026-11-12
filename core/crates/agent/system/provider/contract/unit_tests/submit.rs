use super::*;

#[test]
fn atomic_submit_has_no_settle_gap() {
    assert_eq!(PromptSubmit::Atomic.enter_gap(), Duration::ZERO);
    assert_eq!(PromptSubmit::default(), PromptSubmit::Atomic);
}

const GAP: Duration = Duration::from_millis(100);

#[test]
fn deferred_enter_returns_its_declared_gap() {
    assert_eq!(PromptSubmit::DeferredEnter { gap: GAP }.enter_gap(), GAP);
}
