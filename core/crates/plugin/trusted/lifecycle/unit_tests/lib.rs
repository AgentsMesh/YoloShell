use std::path::{Path, PathBuf};

use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Method};
use yolo_trusted_plugin_manifest::{
    ClientApiContract, Command, Platform, SCHEMA_VERSION, TrustedPluginManifest,
};

fn write_manifest(plugin: &Path, command: &[&str]) {
    let manifest = TrustedPluginManifest {
        schema_version: SCHEMA_VERSION,
        id: "acme.test".into(),
        name: "Test".into(),
        version: "0.1.0".into(),
        description: None,
        platforms: vec![Platform::Linux],
        startup: Some(Command {
            command: command.iter().map(|value| value.to_string()).collect(),
        }),
        actions: Vec::new(),
        web: None,
        api: ClientApiContract::new(HUB_PROTOCOL_VERSION, [Method::Hello], [], []),
    };
    std::fs::write(
        plugin.join("yolo-plugin.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
}

fn context(root: &Path, plugin: PathBuf) -> yolo_trusted_plugin_lifecycle::LifecycleContext {
    yolo_trusted_plugin_lifecycle::LifecycleContext {
        plugin_id: "acme.test".into(),
        root: plugin,
        data: root.join("data"),
        log: root.join("log"),
        hub_socket: PathBuf::from("/tmp/hub.sock"),
        hub_id: None,
        action_id: None,
    }
}

#[test]
fn startup_runs_with_injected_environment_and_bounded_log() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("plugin");
    std::fs::create_dir_all(&plugin).unwrap();
    write_manifest(&plugin, &["/bin/sh", "-c", "test -n \"$YOLO_HUB_SOCKET\""]);
    let outcome =
        yolo_trusted_plugin_lifecycle::run_startup(&context(root.path(), plugin)).unwrap();
    assert!(outcome.success);
    assert!(outcome.log.is_file());
}

#[test]
fn startup_names_a_missing_program() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("plugin");
    std::fs::create_dir_all(&plugin).unwrap();
    write_manifest(&plugin, &["yolo-missing-plugin-runtime"]);
    let error = yolo_trusted_plugin_lifecycle::run_startup(&context(root.path(), plugin))
        .unwrap_err()
        .to_string();
    assert!(error.contains("yolo-missing-plugin-runtime"), "{error}");
    assert!(error.contains("could not start"), "{error}");
}

#[test]
fn log_redacts_hub_endpoint() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("plugin");
    std::fs::create_dir_all(&plugin).unwrap();
    write_manifest(
        &plugin,
        &[
            "/bin/sh",
            "-c",
            "printf '%s %s' \"$YOLO_HUB_SOCKET\" \"$YOLO_HUB_ID\"",
        ],
    );
    let mut context = context(root.path(), plugin);
    context.hub_socket = PathBuf::from("/tmp/secret-hub.sock");
    context.hub_id = Some("11111111-1111-1111-1111-111111111111".parse().unwrap());
    let outcome = yolo_trusted_plugin_lifecycle::run_startup(&context).unwrap();
    let log = std::fs::read_to_string(outcome.log).unwrap();
    assert!(!log.contains("secret-hub.sock"));
    assert!(!log.contains("11111111"));
    assert!(log.contains("<YOLO_HUB_SOCKET>"));
    assert!(log.contains("<YOLO_HUB_ID>"));
}
