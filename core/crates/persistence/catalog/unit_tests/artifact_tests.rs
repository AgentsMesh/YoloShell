use super::*;

#[cfg(unix)]
#[test]
fn hard_link_replacement_after_validation_is_rejected_before_catalog_writes() {
    const OUTSIDE_BYTES: &[u8] = b"external target";

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite");
    std::fs::write(&path, b"").unwrap();
    let snapshot = artifacts::validate_catalog_path(&path).unwrap();
    let original = directory.path().join("original.sqlite");
    std::fs::rename(&path, original).unwrap();
    let outside = directory.path().join("outside.sqlite");
    std::fs::write(&outside, OUTSIDE_BYTES).unwrap();
    std::fs::hard_link(&outside, &path).unwrap();

    let error = artifacts::revalidate_catalog_path(&snapshot).unwrap_err();

    assert!(error.to_string().contains("unexpected hard links"));
    assert_eq!(std::fs::read(outside).unwrap(), OUTSIDE_BYTES);
}
