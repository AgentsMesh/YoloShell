use crate::should_audit;
use yolo_hub_remote_setup_protocol::{RemoteSetupCall, SetupAction};
use yolo_hub_rpc_protocol::Command;

const SHA256_HEX_BYTES: usize = 64;

#[test]
fn profile_mutations_are_audited_and_keep_the_profile_identity() {
    let profile =
        yolo_ssh_profile_protocol::SshProfileRef::new("host.example", "a".repeat(SHA256_HEX_BYTES))
            .unwrap();
    let command = Command::ConnectSshProfile {
        profile,
        credentials: None,
    };
    assert!(should_audit(&command));
    assert_eq!(
        crate::extract::target(&command).as_deref(),
        Some("host.example")
    );
}

#[test]
fn remote_setup_audits_only_mutating_actions_and_keeps_the_profile_identity() {
    let mut call = RemoteSetupCall {
        action: SetupAction::Start,
        request: yolo_hub_remote_setup_protocol_test_support::fixture::request(),
        credentials: None,
    };
    let request_id = call.request.request.fields().request_id;
    let host_id = call.request.request.fields().profile.host_id.clone();
    let command = Command::RemoteSetup { call: call.clone() };
    assert!(should_audit(&command));
    assert_eq!(
        crate::extract::target(&command).as_deref(),
        Some(host_id.as_str())
    );
    assert_eq!(crate::extract::request_id(&command), Some(request_id));

    call.action = SetupAction::Inspect;
    assert!(!should_audit(&Command::RemoteSetup { call }));
}
