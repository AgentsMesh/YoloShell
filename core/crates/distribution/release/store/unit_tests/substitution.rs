use super::{
    constants::*,
    fixture::{self, Fixture},
};
use crate::{
    checkpoint::{Checkpoint, tests::install},
    constants::*,
};
use std::fs;
use uuid::Uuid;
use yolo_distribution_bundle::ArtifactRole;

#[test]
fn replacing_verified_stage_directory_before_rename_does_not_publish_the_replacement() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let (stage, descriptor) = fixture.stage(FIRST_FILL);
    let (substitute, _) = fixture.stage(FIRST_FILL);
    let path = store.stage_path(stage).unwrap();
    let replacement = store.stage_path(substitute).unwrap();
    let displaced = fixture.root.join(DISPLACED_NAME);
    let guard = install(move |point| {
        if point == Checkpoint::BeforeReleaseRename {
            fs::rename(&path, &displaced)?;
            fs::rename(&replacement, &path)?;
        }
        Ok(())
    });
    assert!(
        store
            .publish(stage, &descriptor, None, Uuid::new_v4())
            .is_err()
    );
    drop(guard);
    assert!(store.stage_path(stage).unwrap().exists());
    assert_eq!(store.inspect_release(&descriptor).unwrap(), None);
    assert_eq!(store.inspect().unwrap(), None);
}

#[test]
fn same_bytes_file_substitution_is_rejected_before_rename_and_selector_commit() {
    for point in [
        Checkpoint::BeforeReleaseRename,
        Checkpoint::BeforeSelectionCommit,
    ] {
        let fixture = Fixture::new();
        let store = fixture.store();
        let (stage, descriptor) = fixture.stage(FIRST_FILL);
        let directory = if point == Checkpoint::BeforeReleaseRename {
            store.stage_path(stage).unwrap()
        } else {
            descriptor.release_directory(&fixture.root)
        };
        let artifact = directory.join(ArtifactRole::Runtime.relative_path());
        let displaced = fixture.root.join(DISPLACED_NAME);
        let guard = install(move |current| {
            if current == point {
                fs::rename(&artifact, &displaced)?;
                fs::copy(&displaced, &artifact)?;
            }
            Ok(())
        });
        assert!(
            store
                .publish(stage, &descriptor, None, Uuid::new_v4())
                .is_err()
        );
        drop(guard);
        assert_eq!(store.inspect().unwrap(), None);
    }
}

#[test]
fn selector_same_value_inode_replacement_fails_the_retained_file_identity_cas() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let (stage, first) = fixture.stage(FIRST_FILL);
    let old = store.publish(stage, &first, None, Uuid::new_v4()).unwrap();
    let (stage, second) = fixture.stage(SECOND_FILL);
    let selector = fixture.root.join(CURRENT_FILE_NAME);
    let displaced = fixture.root.join(DISPLACED_NAME);
    let bytes = fs::read(&selector).unwrap();
    let guard = install(move |point| {
        if point == Checkpoint::BeforeSelectionCommit {
            fs::rename(&selector, &displaced)?;
            fixture::write(&selector, &bytes, MANIFEST_MODE);
        }
        Ok(())
    });
    assert!(
        store
            .publish(
                stage,
                &second,
                Some(old.selection().clone()),
                Uuid::new_v4()
            )
            .is_err()
    );
    drop(guard);
    assert_eq!(store.inspect().unwrap().as_ref(), Some(old.selection()));
}
