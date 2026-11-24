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
async fn root_options_before_auxiliary_commands_still_dispatch() {
    let agent = command::run(
        &[LOCAL_PROFILE_OPTION, MISSING_PROFILE, AGENT_COMMAND, LIST],
        None,
    )
    .await;
    assert_eq!(agent.code, FAILURE, "{}", agent.stderr);
    assert!(!agent.stderr.contains("unrecognized subcommand"));
    let update = command::run(
        &[LOCAL_PROFILE_OPTION, MISSING_PROFILE, UPDATE_ENTRY, HELP],
        None,
    )
    .await;
    assert_eq!(update.code, SUCCESS, "{}", update.stderr);
    assert!(update.stdout.contains(UPDATE_USAGE));
}
