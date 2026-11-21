use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Method};
use yolo_plugin_api::{ClientApiContract, Platform};
use yolo_trusted_plugin_manifest::{Command, SCHEMA_VERSION, TrustedPluginManifest};

fn manifest(platforms: Vec<Platform>) -> TrustedPluginManifest {
    TrustedPluginManifest {
        schema_version: SCHEMA_VERSION,
        id: "acme.test".into(),
        name: "Test".into(),
        version: "0.1.0".into(),
        description: None,
        platforms,
        startup: Some(Command {
            command: vec!["true".into()],
        }),
        actions: Vec::new(),
        web: None,
        api: ClientApiContract::new(HUB_PROTOCOL_VERSION, [Method::Hello], [], []),
    }
}

fn parse(
    value: &TrustedPluginManifest,
) -> Result<TrustedPluginManifest, yolo_trusted_plugin_manifest::ManifestError> {
    yolo_trusted_plugin_manifest::parse(&serde_json::to_vec(value).unwrap())
}

#[test]
fn valid_manifest_loads() {
    assert!(parse(&manifest(vec![Platform::Linux])).is_ok());
}

#[test]
fn reserved_and_empty_platforms_are_rejected() {
    let mut value = manifest(vec![Platform::Linux]);
    value.id = "yolo.test".into();
    assert!(parse(&value).is_err());
    value.id = "acme.test".into();
    value.platforms = Vec::new();
    assert!(parse(&value).is_err());
}

#[test]
fn empty_program_is_rejected() {
    let mut value = manifest(vec![Platform::Linux]);
    value.startup = Some(Command {
        command: vec![String::new()],
    });
    assert!(parse(&value).is_err());
}
