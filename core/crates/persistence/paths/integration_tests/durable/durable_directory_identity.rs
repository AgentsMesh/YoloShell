use super::*;

const ORIGINAL: &str = "original";
const DISPLACED: &str = "displaced";

#[test]
fn durable_directory_capture_keeps_live_path_fencing_strict() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ORIGINAL);
    let directory = PrivateDirectory::ensure(&path).unwrap();
    let durable = directory.durable_identity().unwrap();
    let reopened = PrivateDirectory::open(&path).unwrap();
    assert_eq!(reopened.durable_identity().unwrap(), durable);
    std::fs::rename(&path, root.path().join(DISPLACED)).unwrap();
    let replacement = PrivateDirectory::ensure(&path).unwrap();
    assert_ne!(replacement.durable_identity().unwrap(), durable);
    assert!(directory.verify_path().is_err());
    assert!(directory.durable_identity().is_err());
}
