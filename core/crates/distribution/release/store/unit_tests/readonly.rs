use super::{
    constants::*,
    fixture::{self, Fixture},
};
use crate::{
    CURRENT_FILE_NAME, MANIFEST_MODE, ReleaseStore,
    checkpoint::{Checkpoint, tests::install},
};
use std::fs;
use uuid::Uuid;

#[test]
fn readonly_inspection_rejects_selector_inode_replacement_after_release_validation() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let (stage, descriptor) = fixture.stage(FIRST_FILL);
    let installed = store
        .publish(stage, &descriptor, None, Uuid::new_v4())
        .unwrap();
    let selector = fixture.root.join(CURRENT_FILE_NAME);
    let displaced = fixture.root.join(DISPLACED_NAME);
    let bytes = fs::read(&selector).unwrap();
    let reader = ReleaseStore::open_existing(&fixture.root).unwrap().unwrap();
    let guard = install(move |point| {
        if point == Checkpoint::InspectionVerified {
            fs::rename(&selector, &displaced)?;
            fixture::write(&selector, &bytes, MANIFEST_MODE);
        }
        Ok(())
    });
    assert!(reader.inspect_existing().is_err());
    drop(guard);
    let observed = reader.inspect_existing().unwrap().unwrap();
    assert_eq!(observed.selection(), installed.selection());
}
