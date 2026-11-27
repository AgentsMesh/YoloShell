use super::*;
mod catalog {
    pub use yolo_claude_provider::INSTALL_SPEC as CLAUDE;
    pub use yolo_codex_provider::INSTALL_SPEC as CODEX;
    pub use yolo_gemini_provider::INSTALL_SPEC as GEMINI;
    pub use yolo_grok_provider::INSTALL_SPEC as GROK;
    pub use yolo_qwen_provider::INSTALL_SPEC as QWEN;
}

#[test]
fn both_providers_preserve_user_hooks_and_install_idempotently() {
    for provider in [&catalog::CLAUDE, &catalog::CODEX] {
        let input = json!({"userPreference": true, HOOKS: {"SessionStart": [{HOOKS: [{TYPE: COMMAND, COMMAND: "user-handler"}]}]}});
        let original = input.clone();
        let merged = merge(provider, &input).unwrap();
        assert_eq!(input, original);
        assert_eq!(merged["userPreference"], true);
        assert_eq!(
            merged[HOOKS]["SessionStart"][0],
            input[HOOKS]["SessionStart"][0]
        );
        assert_eq!(merge(provider, &merged).unwrap(), merged);
        for event in provider.events {
            assert!(
                merged[HOOKS][event.name]
                    .as_array()
                    .unwrap()
                    .contains(&group(provider, event))
            );
        }
    }
}

#[test]
fn configuration_is_session_scoped_and_never_bypasses_provider_policy() {
    for provider in [&catalog::CLAUDE, &catalog::CODEX] {
        let command = callback_command(provider);
        assert!(command.starts_with("if [ -n "));
        assert!(command.contains(yolo_agent_runtime::NATIVE_HOOK_HOME_ENV));
        assert!(command.contains(yolo_agent_runtime::NATIVE_HOOK_EXECUTABLE_ENV));
        let merged = merge(provider, &json!({})).unwrap();
        assert_eq!(merged.as_object().unwrap().len(), 1);
        assert!(merged.get("features").is_none());
    }
    for &key in catalog::CLAUDE.restricted_keys {
        for value in [json!(true), json!("invalid"), Value::Null] {
            assert!(merge(&catalog::CLAUDE, &json!({key: value})).is_err());
        }
        assert!(merge(&catalog::CLAUDE, &json!({key: false})).is_ok());
    }
}

#[test]
fn malformed_shapes_fail_before_user_data_is_changed() {
    for input in [
        Value::Null,
        json!([]),
        json!({HOOKS: []}),
        json!({HOOKS: {"future": 1}}),
        json!({HOOKS: {"future": [null]}}),
        json!({HOOKS: {"future": [{}]}}),
        json!({HOOKS: {"future": [{HOOKS: [null]}]}}),
    ] {
        let original = input.clone();
        assert!(merge(&catalog::CODEX, &input).is_err());
        assert_eq!(input, original);
    }
}

#[test]
fn qwen_preserves_provider_settings_and_installs_qwen_hook() {
    let input = json!({
        "modelProviders": {"openai": {"baseUrl": "http://127.0.0.1"}},
        "security": {"auth": {"selectedType": "openai"}}
    });
    let merged = merge(&catalog::QWEN, &input).unwrap();
    assert_eq!(merged["modelProviders"], input["modelProviders"]);
    assert_eq!(merged["security"], input["security"]);
    assert_eq!(merge(&catalog::QWEN, &merged).unwrap(), merged);
    let command = merged[HOOKS]["UserPromptSubmit"][0][HOOKS][0][COMMAND]
        .as_str()
        .unwrap();
    assert!(command.contains("provider-hook --provider qwen"));
    assert!(merge(&catalog::QWEN, &json!({"disableAllHooks": true})).is_err());
}

#[test]
fn profile_export_preserves_catalog_identity() {
    let profile = crate::HookProfile::from(&catalog::CODEX);
    assert_eq!(profile.provider, catalog::CODEX.provider);
    assert_eq!(profile.display_name, catalog::CODEX.display_name);
    assert_eq!(profile.subdirectory, catalog::CODEX.subdirectory);
    assert_eq!(profile.file_name, catalog::CODEX.file_name);
    assert_eq!(
        profile.requires_provider_trust,
        catalog::CODEX.requires_provider_trust
    );
    // Providers whose hook file lives below their home expose the relative
    // directory so a client can locate the file without provider knowledge.
    assert_eq!(crate::HookProfile::from(&catalog::GROK).subdirectory, "hooks");
}

#[test]
fn stale_managed_callbacks_are_replaced_and_user_groups_survive() {
    let event = &catalog::GEMINI.events[0];
    let stale = callback_command(&catalog::GEMINI)
        .replace("provider-hook --provider gemini", "gemini-hook");
    let input = json!({HOOKS: {
        event.name: [
            {HOOKS: [{TYPE: COMMAND, COMMAND: stale}]},
            {HOOKS: [{TYPE: COMMAND, COMMAND: "user-handler"}]},
        ]
    }});
    let merged = merge(&catalog::GEMINI, &input).unwrap();
    let groups = merged[HOOKS][event.name].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    assert!(groups.contains(&group(&catalog::GEMINI, event)));
    assert!(groups.iter().any(|value| value[HOOKS][0][COMMAND] == "user-handler"));
    assert!(!groups.iter().any(|value| {
        value[HOOKS][0][COMMAND]
            .as_str()
            .is_some_and(|command| command.contains("gemini-hook"))
    }));
    assert_eq!(merge(&catalog::GEMINI, &merged).unwrap(), merged);
}

#[test]
fn a_mixed_group_loses_only_the_managed_handler() {
    let event = &catalog::CODEX.events[0];
    let stale = callback_command(&catalog::CODEX).replace("provider-hook --provider codex", "codex-hook");
    let input = json!({HOOKS: {
        event.name: [{HOOKS: [
            {TYPE: COMMAND, COMMAND: stale},
            {TYPE: COMMAND, COMMAND: "user-handler"},
        ]}]
    }});
    let merged = merge(&catalog::CODEX, &input).unwrap();
    let groups = merged[HOOKS][event.name].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0][HOOKS].as_array().unwrap().len(), 1);
    assert_eq!(groups[0][HOOKS][0][COMMAND], "user-handler");
    assert!(groups.contains(&group(&catalog::CODEX, event)));
}
