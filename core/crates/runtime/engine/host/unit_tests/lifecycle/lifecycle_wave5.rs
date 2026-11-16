use crate::host::tests::*;
#[test]
fn forced_discard_cannot_replace_lifecycle_authorities() {
    let terminal_id = Uuid::new_v4();
    let creation_capability = Uuid::new_v4();
    let lifecycle = CreationCleanupArchiveRequest {
        terminal_id: terminal_id,
        creation_capability,
    };
    let discard = CreationCleanupDiscardRequest {
        terminal_id: terminal_id,
        creation_capability,
        force: true,
    };

    let lifecycle_result = lifecycle_result(&lifecycle);
    assert_eq!(lifecycle_result.terminal_id, terminal_id);
    assert_eq!(lifecycle_result.creation_capability, creation_capability);
    let discard_result = discard_result(&discard);
    assert_eq!(discard_result.terminal_id, terminal_id);
    assert_eq!(discard_result.creation_capability, creation_capability);
}
