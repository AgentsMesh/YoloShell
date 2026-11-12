use crate::coverage_json::{COUNT_INDEX, FALSE_COUNT_INDEX, LINE_END_INDEX};
use crate::instance_fixture::*;
use crate::signature;

#[test]
fn static_map_signature_excludes_execution_counts() {
    let baseline = signature::mapping(&function(), BRANCH_OUTCOMES).unwrap();
    let mut changed = function();
    changed.count = u64::default();
    changed.regions[FILE_INDEX as usize][COUNT_INDEX] = u64::default();
    changed.branches[FILE_INDEX as usize][COUNT_INDEX] = u64::default();
    changed.branches[FILE_INDEX as usize][FALSE_COUNT_INDEX] = u64::default();
    assert_eq!(
        signature::mapping(&changed, BRANCH_OUTCOMES).unwrap(),
        baseline
    );
    changed.regions[FILE_INDEX as usize][LINE_END_INDEX] += SINGLE_METRIC;
    assert_ne!(
        signature::mapping(&changed, BRANCH_OUTCOMES).unwrap(),
        baseline
    );
    assert_ne!(
        signature::mapping(&function(), SINGLE_METRIC).unwrap(),
        baseline
    );
}

#[test]
fn logical_identity_binds_names_and_every_filename() {
    let files = vec![SOURCE.to_owned()];
    let identity = signature::identity(FUNCTION, &files).unwrap();
    assert_ne!(
        signature::identity(OTHER_FUNCTION, &files).unwrap(),
        identity
    );
    assert_ne!(
        signature::identity(FUNCTION, &[OTHER_SOURCE.to_owned()]).unwrap(),
        identity
    );
    assert!(signature::identity("", &files).is_err());
}
