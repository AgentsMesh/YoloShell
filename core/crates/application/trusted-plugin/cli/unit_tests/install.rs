use crate::constants::SCHEMA_VERSION_FIELD;
use crate::install_registry::load_spec;
use serde_json::json;
use std::path::Path;
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Method};
use yolo_trusted_plugin_manifest::{ClientApiContract, Platform, SCHEMA_VERSION, TrustedPluginManifest};

fn manifest(id: &str, version: &str) -> TrustedPluginManifest {
    TrustedPluginManifest {
        schema_version: SCHEMA_VERSION,
        id: id.into(),
        name: "Acme Test".into(),
        version: version.into(),
        description: Some("Test plugin".into()),
        platforms: vec![Platform::Linux],
        startup: None,
        actions: vec![],
        web: None,
        api: ClientApiContract::new(HUB_PROTOCOL_VERSION, [Method::Hello], [], []),
    }
}

fn write_spec(
    root: &Path,
    id: &str,
    version: &str,
    commit: &str,
    source_path: &str,
    manifest: TrustedPluginManifest,
) {
    let directory = root.join("Specs").join(id);
    std::fs::create_dir_all(&directory).unwrap();
    let document = json!({
        (SCHEMA_VERSION_FIELD): 1,
        "id": id,
        "version": version,
        "name": "Acme Test",
        "description": "Test plugin",
        "source": {
            "repository": "https://github.com/example/plugin",
            "path": source_path,
            "tag": "v0.1.0",
            "commit": commit,
        },
        "manifest": manifest,
    });
    std::fs::write(
        directory.join(format!("{version}.json")),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
}

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn selects_the_highest_semver_and_honors_an_explicit_version() {
    let root = tempfile::tempdir().unwrap();
    for version in ["0.9.0", "0.10.0", "0.2.0"] {
        write_spec(root.path(), "acme.test", version, COMMIT, ".", manifest("acme.test", version));
    }
    let latest = load_spec(root.path(), "acme.test", None).unwrap();
    assert_eq!(latest.version, "0.10.0");
    let pinned = load_spec(root.path(), "acme.test", Some("0.2.0")).unwrap();
    assert_eq!(pinned.version, "0.2.0");
    assert!(load_spec(root.path(), "acme.test", Some("0.3.0")).is_err());
}

#[test]
fn rejects_traversal_bad_commit_and_manifest_mismatch() {
    let root = tempfile::tempdir().unwrap();
    write_spec(root.path(), "acme.test", "0.1.0", COMMIT, "../escape", manifest("acme.test", "0.1.0"));
    assert!(load_spec(root.path(), "acme.test", None).is_err());

    let root = tempfile::tempdir().unwrap();
    write_spec(root.path(), "acme.test", "0.1.0", "ABC", ".", manifest("acme.test", "0.1.0"));
    assert!(load_spec(root.path(), "acme.test", None).is_err());

    let root = tempfile::tempdir().unwrap();
    write_spec(root.path(), "acme.test", "0.1.0", COMMIT, ".", manifest("other.test", "0.1.0"));
    assert!(load_spec(root.path(), "acme.test", None).is_err());
}

#[test]
fn rejects_reserved_and_malformed_ids_before_reading_the_registry() {
    let root = tempfile::tempdir().unwrap();
    let error = load_spec(root.path(), "yolo.web", None).unwrap_err().to_string();
    assert!(error.contains("web.client"), "{error}");
    assert!(load_spec(root.path(), "../escape", None).is_err());
    assert!(load_spec(root.path(), "WEB", None).is_err());
}
