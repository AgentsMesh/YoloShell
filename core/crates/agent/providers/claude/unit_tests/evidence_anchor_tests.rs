use super::{constants, evidence_anchor_support::*, evidence_path::AnchoredEvidence};
use std::{fs, os::unix::fs::symlink};

const EXTRA_LINK: &str = "extra-transcript-link";

#[test]
fn stable_named_evidence_retains_its_exact_descriptor_and_durable_identity() {
    let fixture = Fixture::new();
    let mut anchor = fixture.open();
    assert!(anchor.verify().unwrap());
    let original = anchor.durable_object_identity().unwrap().unwrap();
    assert_retained_bytes(&mut anchor);
    assert_eq!(anchor.durable_object_identity().unwrap(), Some(original));
    assert!(anchor.verify().unwrap());
}

#[test]
fn disappearance_of_home_or_project_entry_revokes_path_proof_even_with_readable_old_descriptor() {
    for remove_home in [false, true] {
        let fixture = Fixture::new();
        let mut anchor = fixture.open();
        let retained_identity = anchor.durable_object_identity().unwrap();
        let original = if remove_home {
            &fixture.root
        } else {
            &fixture.scope
        };
        let displaced = original.parent().unwrap().join(DISPLACED);
        fs::rename(original, &displaced).unwrap();
        assert!(!anchor.verify().unwrap());
        assert_eq!(anchor.durable_object_identity().unwrap(), retained_identity);
        assert_retained_bytes(&mut anchor);
        fs::rename(&displaced, original).unwrap();
        assert!(anchor.verify().unwrap());
    }
}

#[test]
fn adding_a_hardlink_after_open_revokes_the_snapshot_without_losing_the_retained_bytes() {
    let fixture = Fixture::new();
    let mut anchor = fixture.open();
    let link = fixture.scope.join(EXTRA_LINK);
    fs::hard_link(&fixture.transcript, &link).unwrap();
    assert!(!anchor.verify().unwrap());
    assert_retained_bytes(&mut anchor);
    // New inspection may establish a new snapshot only after the alias is gone.
    assert!(
        AnchoredEvidence::open(&fixture.root, &fixture.transcript)
            .unwrap()
            .is_none()
    );
    fs::remove_file(link).unwrap();
    assert!(fixture.open().verify().unwrap());
}

#[test]
fn replacing_the_named_transcript_with_a_directory_or_symlink_cannot_reauthorize_the_old_descriptor()
 {
    for symbolic in [false, true] {
        let fixture = Fixture::new();
        let mut anchor = fixture.open();
        let retained = anchor.durable_object_identity().unwrap();
        let displaced = fixture.scope.join(DISPLACED);
        fs::rename(&fixture.transcript, &displaced).unwrap();
        if symbolic {
            symlink(&displaced, &fixture.transcript).unwrap();
        } else {
            fs::create_dir(&fixture.transcript).unwrap();
        }
        assert!(!anchor.verify().unwrap());
        assert_eq!(anchor.durable_object_identity().unwrap(), retained);
        assert_retained_bytes(&mut anchor);
        assert!(
            AnchoredEvidence::open(&fixture.root, &fixture.transcript)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn an_ancestor_replaced_by_a_regular_file_preserves_the_real_inspection_error() {
    let fixture = Fixture::new();
    let mut anchor = fixture.open();
    let displaced = fixture.workspace.parent().unwrap().join(DISPLACED);
    fs::rename(&fixture.workspace, &displaced).unwrap();
    fs::write(&fixture.workspace, PRESERVED_BYTES).unwrap();
    let error = anchor.verify().unwrap_err();
    assert_eq!(error.to_string(), constants::DATA_HOME_INSPECT_ERROR);
    let io = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<std::io::Error>())
        .unwrap();
    assert_eq!(io.raw_os_error(), Some(libc::ENOTDIR));
    assert_retained_bytes(&mut anchor);
    assert_eq!(fs::read(&fixture.workspace).unwrap(), PRESERVED_BYTES);
}
