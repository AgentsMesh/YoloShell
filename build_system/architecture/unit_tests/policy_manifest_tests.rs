use yolo_architecture_syntax::starlark_baseline::tuple_entries;

#[test]
fn every_manifest_dispatch_has_a_checker_call() {
    let runfiles = crate::locate_runfiles_overlay()
        .unwrap()
        .expect("architecture tests run with their runfiles");
    let manifest = std::fs::read_to_string(runfiles.join("build_system/architecture/policies.bzl"))
        .unwrap();
    let checker = std::fs::read_to_string(runfiles.join("build_system/architecture/src/checker.rs"))
        .unwrap();
    let dispatches = tuple_entries(&manifest, "POLICY_DISPATCHES", 2).unwrap();
    assert!(!dispatches.is_empty());
    let missing = dispatches
        .iter()
        .filter(|entry| !checker.contains(&entry.values[1]))
        .map(|entry| format!("{}: {}", entry.values[0], entry.values[1]))
        .collect::<Vec<_>>();
    assert!(missing.is_empty(), "{missing:?}");
}
