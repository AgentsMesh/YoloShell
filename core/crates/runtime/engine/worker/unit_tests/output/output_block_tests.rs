use super::*;

#[test]
fn causes_remain_blocked_until_each_resolving_path_clears_its_own_cause() {
    let block = OutputBlock::default();
    assert!(!block.blocked());
    assert!(block.mark(OutputBlockCause::FinalDrainDeadline));
    assert!(!block.mark(OutputBlockCause::FinalDrainDeadline));
    assert!(block.mark(OutputBlockCause::ExitPersistence));

    block.clear(OutputBlockCause::FinalDrainDeadline);
    assert!(block.blocked());
    assert_eq!(
        block.causes_for_test().collect::<Vec<_>>(),
        vec![OutputBlockCause::ExitPersistence]
    );

    block.clear(OutputBlockCause::ExitPersistence);
    assert!(!block.blocked());
    assert!(block.causes_for_test().next().is_none());
}
