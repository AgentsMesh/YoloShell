use std::fs;

use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Method};
use yolo_trusted_plugin_manifest::{
    ClientApiContract, Command, Platform, SCHEMA_VERSION, TrustedPluginManifest,
};

#[test]
fn links_revalidates_and_rejects_duplicate_ids() {
    let root = tempfile::tempdir().unwrap();
    let plugin = root.path().join("plugin");
    fs::create_dir(&plugin).unwrap();
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
        actions: Vec::new(),
        web: None,
        api: ClientApiContract::new(HUB_PROTOCOL_VERSION, [Method::Hello], [], []),
    };
    fs::write(
        plugin.join("yolo-plugin.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let paths = yolo_trusted_plugin_registry::RegistryPaths::new(
        root.path().join("config"),
        root.path().join("data"),
        root.path().join("state"),
    );
    let mut registry = yolo_trusted_plugin_registry::Registry::load(&paths).unwrap();
    registry.link(&plugin, &paths).unwrap();
    assert!(registry.revalidate("acme.test").is_ok());
    assert!(registry.link(&plugin, &paths).is_err());
    fs::write(plugin.join("changed"), "x").unwrap();
    assert!(registry.revalidate("acme.test").is_err());
}
