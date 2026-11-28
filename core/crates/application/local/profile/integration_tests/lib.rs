use yolo_local_profile::LocalProfile;

const PROFILE_FILE: &str = "profile.json";

#[test]
fn private_profile_load_preserves_explicit_paths_and_rejects_invalid_documents() {
    let root = tempfile::tempdir().unwrap();
    let profile = LocalProfile::from_application_support(root.path()).unwrap();
    let path = root.path().join(PROFILE_FILE);
    yolo_paths::atomic_write_json(&path, &profile).unwrap();
    assert_eq!(LocalProfile::load(&path).unwrap(), profile);
    let mut bad = serde_json::to_value(&profile).unwrap();
    bad["unknown"] = serde_json::Value::Bool(true);
    yolo_paths::atomic_write_json(&path, &bad).unwrap();
    assert!(LocalProfile::load(&path).is_err());
    assert!(!profile.state.exists());
    assert!(!profile.host_home.exists());
}
