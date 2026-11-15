use super::{command, constants::*};

#[tokio::test]
async fn single_image_exposes_public_protocol_without_a_tty() {
    let protocol = command::run(&[PROTOCOL, DESCRIBE], None).await;
    assert_eq!(protocol.code, SUCCESS, "{}", protocol.stderr);
    let document: serde_json::Value = serde_json::from_str(&protocol.stdout).unwrap();
    assert_eq!(document["protocol"], yolo_hub_rpc_descriptor::PROTOCOL_NAME);
    assert!(
        document["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["method"] == "attach")
    );
}

#[tokio::test]
async fn protocol_discovery_is_strict() {
    for arguments in [
        &[PROTOCOL, "unknown"][..],
        &[PROTOCOL, DESCRIBE, "extra"][..],
    ] {
        let result = command::run(arguments, None).await;
        assert_eq!(result.code, USAGE);
        assert!(result.stdout.is_empty());
    }
}

#[tokio::test]
async fn web_command_is_dispatched_without_entering_the_tui() {
    let help = command::run(&[WEB_COMMAND, HELP], None).await;
    assert_eq!(help.code, SUCCESS, "{}", help.stderr);
    assert!(help.stdout.contains(WEB_CLIENT_KEY_OPTION));
    assert!(help.stdout.contains(WEB_ORIGIN_OPTION));
    let missing = command::run(&[WEB_COMMAND], None).await;
    assert_eq!(missing.code, USAGE);
    assert!(missing.stderr.contains(WEB_PROFILE_OPTION));
}
