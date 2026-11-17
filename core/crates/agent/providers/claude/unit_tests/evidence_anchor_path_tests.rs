use super::{constants, evidence_anchor_support::*, evidence_path::AnchoredEvidence};
use std::{
    ffi::OsString,
    fs,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
};

const FOREIGN_HOME: &str = "foreign-home";
const WRONG_EVIDENCE_DIRECTORY: &str = crate::agent::claude::constants::INVALID_EVIDENCE_DIR;
const EXTRA_COMPONENT: &str = "nested";
const PARENT_COMPONENT: &str = "..";
const NUL_BYTE: u8 = 0;
const NUL_LEAF: &[u8] = b"invalid\0.jsonl";

#[test]
fn invalid_root_or_transcript_shape_is_rejected_before_obtaining_evidence() {
    let fixture = Fixture::new();
    let relative = PathBuf::from(DATA_HOME);
    let parent = fixture.root.join(PARENT_COMPONENT);
    let paths = [
        (
            relative.clone(),
            relative
                .join(constants::EVIDENCE_DIR)
                .join(PROJECT_SCOPE)
                .join(TRANSCRIPT),
        ),
        (
            parent.clone(),
            parent
                .join(constants::EVIDENCE_DIR)
                .join(PROJECT_SCOPE)
                .join(TRANSCRIPT),
        ),
        (
            fixture.root.clone(),
            fixture
                .workspace
                .join(FOREIGN_HOME)
                .join(constants::EVIDENCE_DIR)
                .join(PROJECT_SCOPE)
                .join(TRANSCRIPT),
        ),
        (
            fixture.root.clone(),
            fixture.root.join(PROJECT_SCOPE).join(TRANSCRIPT),
        ),
        (
            fixture.root.clone(),
            fixture.scope.join(EXTRA_COMPONENT).join(TRANSCRIPT),
        ),
        (
            fixture.root.clone(),
            fixture
                .root
                .join(WRONG_EVIDENCE_DIRECTORY)
                .join(PROJECT_SCOPE)
                .join(TRANSCRIPT),
        ),
        (
            fixture.root.clone(),
            fixture.scope.join(OsString::from_vec(NUL_LEAF.to_vec())),
        ),
    ];
    for (root, path) in paths {
        assert!(AnchoredEvidence::open(&root, &path).unwrap().is_none());
    }
    assert_eq!(fs::read(&fixture.transcript).unwrap(), PRESERVED_BYTES);
    assert!(fixture.open().verify().unwrap());
}

#[test]
fn nul_in_an_absolute_data_home_keeps_the_specific_configuration_error() {
    let fixture = Fixture::new();
    let mut bytes = fixture.root.as_os_str().as_bytes().to_vec();
    bytes.push(NUL_BYTE);
    let root = PathBuf::from(OsString::from_vec(bytes));
    let path = root
        .join(constants::EVIDENCE_DIR)
        .join(PROJECT_SCOPE)
        .join(TRANSCRIPT);
    let error = AnchoredEvidence::open(&root, &path).err().unwrap();
    assert_eq!(error.to_string(), constants::DATA_HOME_NUL_ERROR);
    assert!(
        error
            .chain()
            .any(|cause| cause.downcast_ref::<std::ffi::NulError>().is_some())
    );
    assert_eq!(fs::read(&fixture.transcript).unwrap(), PRESERVED_BYTES);
}
