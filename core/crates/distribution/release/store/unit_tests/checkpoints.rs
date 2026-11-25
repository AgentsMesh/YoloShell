use super::{constants::*, fixture::Fixture};
use crate::checkpoint::{Checkpoint, tests::install};
use uuid::Uuid;

#[test]
fn every_publication_checkpoint_preserves_inspectable_state_and_exact_retry_identity() {
    for point in [
        Checkpoint::StageVerified,
        Checkpoint::BeforeReleaseRename,
        Checkpoint::AfterReleaseRename,
        Checkpoint::ReleaseSynced,
        Checkpoint::BeforeSelectionCommit,
        Checkpoint::AfterSelectionCommit,
    ] {
        let fixture = Fixture::new();
        let store = fixture.store();
        let (stage, descriptor) = fixture.stage(FIRST_FILL);
        let operation = Uuid::new_v4();
        let guard = install(move |current| {
            anyhow::ensure!(current != point, LOST_ACK);
            Ok(())
        });
        assert!(store.publish(stage, &descriptor, None, operation).is_err());
        drop(guard);
        let selected = store.inspect().unwrap();
        if point == Checkpoint::AfterSelectionCommit {
            assert_eq!(selected.as_ref().unwrap().operation_id(), operation);
            assert_eq!(selected.as_ref().unwrap().build_id(), descriptor.build_id());
        } else {
            assert_eq!(selected, None);
        }
        let before_rename = matches!(
            point,
            Checkpoint::StageVerified | Checkpoint::BeforeReleaseRename
        );
        assert_eq!(store.stage_path(stage).unwrap().exists(), before_rename);
        assert_eq!(
            store.inspect_release(&descriptor).unwrap().is_some(),
            !before_rename
        );
        let installed = store.publish(stage, &descriptor, None, operation).unwrap();
        assert_eq!(installed.selection().operation_id(), operation);
        assert_eq!(
            store.inspect().unwrap().as_ref(),
            Some(installed.selection())
        );
        let replay = store.publish(stage, &descriptor, None, operation).unwrap();
        assert_eq!(replay.selection(), installed.selection());
    }
}

#[test]
fn a_late_retry_cannot_replace_a_successor_after_lost_acknowledgement() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let (first_stage, first) = fixture.stage(FIRST_FILL);
    let first_operation = Uuid::new_v4();
    let guard = install(|point| {
        anyhow::ensure!(point != Checkpoint::AfterSelectionCommit, LOST_ACK);
        Ok(())
    });
    assert!(
        store
            .publish(first_stage, &first, None, first_operation)
            .is_err()
    );
    drop(guard);
    let previous = store.inspect().unwrap();
    let (second_stage, second) = fixture.stage(SECOND_FILL);
    let installed = store
        .publish(second_stage, &second, previous, Uuid::new_v4())
        .unwrap();
    assert!(
        store
            .publish(first_stage, &first, None, first_operation)
            .is_err()
    );
    assert_eq!(
        store.inspect().unwrap().as_ref(),
        Some(installed.selection())
    );
    assert!(store.inspect_release(&first).unwrap().is_some());
}
