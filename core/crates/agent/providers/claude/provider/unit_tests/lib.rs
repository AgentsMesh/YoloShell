use crate::{CONVERSATION_TOKEN, DISPLAY_NAME, INSTALL_TOKEN, identity, install, runtime};
use yolo_agent_provider_contract::AuthorityKind;
use yolo_agent_provider_runtime::HookClient;

#[test]
fn identity_and_manifest_definition_agree() {
    let definition = identity::definition();
    assert_eq!(definition.id(), identity::PROVIDER_ID);
    assert_eq!(definition.kind(), &identity::kind());
    assert_eq!(identity::kind().as_str(), identity::KIND);
}

#[test]
fn install_spec_is_owned_by_the_provider_package() {
    let spec = install::install_spec();
    assert_eq!(spec.provider, INSTALL_TOKEN);
    assert_eq!(spec.display_name, DISPLAY_NAME);
    assert_eq!(
        spec.environment_name,
        yolo_claude::claude::constants::DATA_HOME_ENV
    );
    assert_eq!(
        spec.directory_name,
        yolo_claude::claude::constants::DEFAULT_DATA_HOME_DIR
    );
    assert_eq!(spec.hook_arguments, "provider-hook --provider claude");
    assert!(spec.managed_hook_commands.contains(&"claude-hook"));
    assert!(!spec.events.is_empty());
    assert!(!spec.requires_provider_trust);
}

#[test]
fn runtime_row_matches_the_identity_and_ports() {
    let provider = runtime();
    let descriptor = provider.descriptor();
    assert_eq!(descriptor.manifest_id(), identity::PROVIDER_ID);
    assert_eq!(descriptor.kind(), &identity::kind());
    assert_eq!(descriptor.install_token(), INSTALL_TOKEN);
    assert_eq!(descriptor.conversation_token(), CONVERSATION_TOKEN);
    assert_eq!(descriptor.display_name(), DISPLAY_NAME);
    assert_eq!(descriptor.authority_kind(), AuthorityKind::ClaudeGeneration);
    assert_eq!(provider.hook_client(), HookClient::ClaudeNative);
    assert!(provider.hook_codec().is_none());
    assert_eq!(
        provider.install_spec().expect("install spec").provider,
        INSTALL_TOKEN
    );
    let source = provider
        .conversation_source()
        .expect("conversation source");
    assert_eq!(source.kind(), identity::kind());
    assert_eq!(source.conversation_token().as_str(), CONVERSATION_TOKEN);
}
