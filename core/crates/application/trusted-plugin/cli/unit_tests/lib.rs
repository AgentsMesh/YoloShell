use std::path::PathBuf;

use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Method};
use yolo_trusted_plugin_manifest::{
    Action, ClientApiContract, Command, Platform, SCHEMA_VERSION, TrustedPluginManifest,
};
use yolo_trusted_plugin_registry::{Registry, RegistryPaths, StartupStatus};

#[test]
fn action_does_not_replace_last_startup_status() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("plugin");
    std::fs::create_dir_all(&plugin).unwrap();
    let manifest = TrustedPluginManifest {
        schema_version: SCHEMA_VERSION,
        id: "acme.test".into(),
        name: "Test".into(),
        version: "0.1.0".into(),
        description: None,
        platforms: vec![Platform::Linux],
        startup: Some(Command {
            command: vec!["true".into()],
        }),
        actions: vec![Action {
            id: "status".into(),
            title: "Status".into(),
            command: vec!["true".into()],
        }],
        web: None,
        api: ClientApiContract::new(HUB_PROTOCOL_VERSION, [Method::Hello], [], []),
    };
    std::fs::write(
        plugin.join("yolo-plugin.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let paths = RegistryPaths::new(
        root.path().join("config"),
        root.path().join("data"),
        root.path().join("state"),
    );
    let mut registry = Registry::load(&paths).unwrap();
    registry.link(&plugin, &paths).unwrap();
    let status = StartupStatus {
        success: true,
        finished_at_unix_ms: 7,
    };
    registry.set_startup("acme.test", status, &paths).unwrap();
    let endpoint = crate::execution::Endpoint {
        socket: PathBuf::from("/tmp/hub.sock"),
        hub: None,
    };
    crate::execution::execute(
        &mut registry,
        &paths,
        "acme.test",
        &endpoint,
        Some("status"),
    )
    .unwrap();
    assert_eq!(
        registry.get("acme.test").unwrap().last_startup,
        Some(status)
    );
}
