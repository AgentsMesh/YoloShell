use super::*;
use std::collections::BTreeSet;
use yolo_agent_provider_contract::{AuthorityKind, InteractionSemantics};

#[test]
fn every_provider_row_names_one_install_spec_one_codec_and_one_source() {
    let runtime = builtin_provider_runtime();
    let mut identities = BTreeSet::new();
    for provider in runtime.runtimes() {
        let descriptor = provider.descriptor();
        assert!(
            identities.insert((
                descriptor.manifest_id().to_owned(),
                descriptor.kind().clone(),
                descriptor.install_token().to_owned(),
                descriptor.conversation_token().to_owned(),
            )),
            "duplicate provider identity: {descriptor:?}"
        );

        let spec = provider.install_spec().expect("install spec");
        assert_eq!(spec.provider, descriptor.install_token());
        assert_eq!(spec.display_name, descriptor.display_name());

        let source = provider.conversation_source().expect("conversation source");
        assert_eq!(source.kind(), *descriptor.kind());
        assert_eq!(
            source.conversation_token().as_str(),
            descriptor.conversation_token()
        );
        match descriptor.authority_kind() {
            AuthorityKind::GenericHookGeneration => {
                let codec = provider.hook_codec().expect("generic hook codec");
                assert_eq!(codec.kind(), *descriptor.kind());
                assert_eq!(codec.profile().provider, descriptor.install_token());
            }
            AuthorityKind::CodexOccurrence | AuthorityKind::ClaudeGeneration => {
                assert!(provider.hook_codec().is_none());
            }
            AuthorityKind::None => panic!("compiled provider must declare an authority"),
        }

        assert!(
            yolo_plugin_provider_catalog::get(descriptor.manifest_id()).is_some(),
            "provider manifest is missing: {}",
            descriptor.manifest_id()
        );
    }
}

#[test]
fn typed_interaction_semantics_require_the_native_bridge_authority() {
    for provider in builtin_provider_runtime().runtimes() {
        let typed = provider.interaction_semantics() == InteractionSemantics::Typed;
        let bridged =
            provider.descriptor().authority_kind() == AuthorityKind::ClaudeGeneration;
        assert_eq!(
            typed,
            bridged,
            "{} declares interaction semantics {:?}",
            provider.descriptor().kind().as_str(),
            provider.interaction_semantics()
        );
    }
}

#[test]
fn install_specs_and_projection_policies_cover_every_row() {
    let runtime = builtin_provider_runtime();
    let specs = builtin_install_specs();
    assert_eq!(specs.len(), runtime.len());
    for spec in specs {
        assert!(runtime.install_spec(spec.provider).is_some());
    }
    let policies = builtin_projection_policies();
    for provider in runtime.runtimes() {
        assert_eq!(
            policies.policy(provider.descriptor().kind()),
            provider.descriptor().projection_policy()
        );
    }
}

#[test]
fn conversation_sources_are_derived_from_the_rows() {
    let sources = builtin_conversation_sources();
    let runtime = builtin_provider_runtime();
    for provider in runtime.runtimes() {
        let token = provider.descriptor().conversation_token();
        let token = yolo_conversation_protocol::ConversationProvider::new(token).unwrap();
        assert!(sources.source_for_token(&token).is_some(), "{token:?}");
    }
}

#[test]
fn launch_policies_are_derived_from_every_provider_row() {
    let runtime = builtin_provider_runtime();
    let policies = builtin_launch_policies();
    for provider in runtime.runtimes() {
        let kind = provider.descriptor().kind();
        assert_eq!(
            policies.command(kind.as_str()),
            provider.launch_command(),
            "launch command mismatch for {kind:?}"
        );
    }
}
