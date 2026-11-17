use super::{constants::*, fixture::Fixture};
use std::fs;
use uuid::Uuid;
use yolo_distribution_bundle::{ArtifactRole, RELEASES_DIRECTORY};
use yolo_distribution_release_store::ReleaseStore;

#[test]
fn whole_release_publication_is_durable_idempotent_and_paths_stay_pinned_across_upgrades() {
    let fixture = Fixture::new();
    let store = fixture.store();
    assert_eq!(store.root(), fixture.root);
    assert_eq!(store.inspect().unwrap(), None);
    let (stage, first) = fixture.stage(FIRST_FILL);
    let operation = Uuid::new_v4();
    let installed = store.publish(stage, &first, None, operation).unwrap();
    assert_eq!(installed.descriptor(), &first);
    assert_eq!(
        installed.release_directory(),
        first.release_directory(&fixture.root)
    );
    assert!(!store.stage_path(stage).unwrap().exists());
    let reopened = ReleaseStore::open(&fixture.root).unwrap();
    assert_eq!(
        reopened.inspect().unwrap().as_ref(),
        Some(installed.selection())
    );
    assert_eq!(
        reopened.inspect_release(&first).unwrap().as_deref(),
        Some(installed.release_directory())
    );
    let replay = reopened.publish(stage, &first, None, operation).unwrap();
    assert_eq!(replay.selection(), installed.selection());
    let old_paths = ArtifactRole::ALL.map(|role| installed.executable(role));
    let old_bytes = old_paths.each_ref().map(|path| fs::read(path).unwrap());
    let (stage, second) = fixture.stage(SECOND_FILL);
    assert!(
        store
            .publish(
                stage,
                &second,
                Some(installed.selection().clone()),
                operation
            )
            .is_err()
    );
    let upgraded = store
        .publish(
            stage,
            &second,
            Some(installed.selection().clone()),
            Uuid::new_v4(),
        )
        .unwrap();
    assert!(upgraded.selection().revision() > installed.selection().revision());
    assert_ne!(upgraded.release_directory(), installed.release_directory());
    for ((role, path), bytes) in ArtifactRole::ALL.into_iter().zip(old_paths).zip(old_bytes) {
        assert_eq!(installed.executable(role), path);
        assert_eq!(fs::read(path).unwrap(), bytes);
        assert_ne!(installed.executable(role), upgraded.executable(role));
    }
    assert_eq!(
        store.inspect().unwrap().as_ref(),
        Some(upgraded.selection())
    );
    assert_eq!(
        fs::read_dir(fixture.root.join(RELEASES_DIRECTORY))
            .unwrap()
            .count(),
        RACE_PARTICIPANTS
    );
}

#[test]
fn preexisting_complete_release_can_be_selected_but_partial_release_is_never_repaired() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let (stage, first) = fixture.stage(FIRST_FILL);
    let initial = store.publish(stage, &first, None, Uuid::new_v4()).unwrap();
    let (stage, second) = fixture.stage(SECOND_FILL);
    let stage_path = store.stage_path(stage).unwrap();
    let release = second.release_directory(&fixture.root);
    fs::rename(&stage_path, &release).unwrap();
    assert_eq!(
        store.inspect_release(&second).unwrap(),
        Some(release.clone())
    );
    let artifact = release.join(ArtifactRole::Runtime.relative_path());
    fs::remove_file(&artifact).unwrap();
    let (repair_stage, _) = fixture.stage(SECOND_FILL);
    assert!(
        store
            .publish(
                repair_stage,
                &second,
                Some(initial.selection().clone()),
                Uuid::new_v4()
            )
            .is_err()
    );
    assert!(!artifact.exists());
    assert!(store.stage_path(repair_stage).unwrap().exists());
    assert!(store.inspect_release(&second).is_err());
    assert_eq!(store.inspect().unwrap().as_ref(), Some(initial.selection()));
}

#[test]
fn a_complete_unselected_release_is_reused_without_requiring_its_consumed_stage() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let (stage, descriptor) = fixture.stage(FIRST_FILL);
    fs::rename(
        store.stage_path(stage).unwrap(),
        descriptor.release_directory(&fixture.root),
    )
    .unwrap();
    let installed = store
        .publish(stage, &descriptor, None, Uuid::new_v4())
        .unwrap();
    assert_eq!(
        store.inspect().unwrap().as_ref(),
        Some(installed.selection())
    );
}

#[test]
fn payloads_crossing_multiple_hash_chunks_publish_with_exact_bytes() {
    let fixture = Fixture::new();
    let payloads = ArtifactRole::ALL.map(|role| {
        let mut bytes = super::fixture::payload(role, FIRST_FILL);
        bytes.resize(STREAMING_PAYLOAD_BYTES, FIRST_FILL);
        bytes
    });
    let (stage, descriptor) = fixture.stage_payloads(payloads.clone());
    let installed = fixture
        .store()
        .publish(stage, &descriptor, None, Uuid::new_v4())
        .unwrap();
    for (role, expected) in ArtifactRole::ALL.into_iter().zip(payloads) {
        assert_eq!(fs::read(installed.executable(role)).unwrap(), expected);
    }
}
