use crate::host::tests::*;
use super::*;

#[test]
fn agent_authority_must_match_the_observed_runtime_epoch() {
    let observed = Uuid::new_v4();
    let replacement = Uuid::new_v4();
    assert!(runtime_epochs_pair(Some(observed), Some(observed)));
    assert!(!runtime_epochs_pair(Some(observed), Some(replacement)));
    assert!(runtime_epochs_pair(None, Some(observed)));
    assert!(!runtime_epochs_pair(Some(observed), None));
}
