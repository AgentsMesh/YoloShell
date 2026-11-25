use super::{source, violations};

const FAMILIES: &[&str] = &[
    "core/crates/agent",
    "core/crates/application",
    "core/crates/conversation",
    "core/crates/distribution",
    "core/crates/hub",
    "core/crates/persistence",
    "core/crates/plugin",
    "core/crates/process",
    "core/crates/presentation",
    "core/crates/runtime",
    "core/crates/transport",
    "core/crates/terminal",
    "clients/tui",
    "build_system/architecture",
];
const BUILD_NAMES: &[&str] = &["BUILD", "BUILD.bazel"];
const ALIAS_CONTENT: &str =
    "alias(name = \"library\", actual = \"//core/crates/agent/contract:yolo_agent_contract\")";

fn assert_rejected(path: &str) {
    let result = violations(&[source(path)], &[]);
    assert_eq!(result.len(), 1, "{path}: {result:?}");
    assert!(result[0].message.contains("directory hierarchy"));
}

#[test]
fn component_packages_cannot_flatten_domain_and_function_names() {
    for path in [
        "core/crates/hub/client_access_store/BUILD.bazel",
        "core/crates/hub/client/access_store/BUILD.bazel",
        "core/crates/application/broker_supervision_state/BUILD.bazel",
        "core/crates/application/broker/supervision_state/BUILD.bazel",
        "clients/tui/terminal_queries/BUILD.bazel",
        "clients/tui/client/control_protocol/BUILD",
        "build_system/architecture/process_signal_graph/BUILD.bazel",
        "build_system/architecture/process/signal/retained_policy/BUILD.bazel",
        "build_system/architecture/repository_literal_policy/BUILD.bazel",
        "core/crates/agent/providers/claude_hook_bridge/BUILD.bazel",
        "core/crates/agent/providers/codex_hook_contract/BUILD.bazel",
        "core/crates/agent/system/hooks_runtime/BUILD.bazel",
        "core/crates/agent/system/hook_installation/BUILD.bazel",
        "core/crates/agent/system/control_journal/BUILD.bazel",
        "core/crates/agent/system/binding_io/BUILD.bazel",
        "core/crates/agent/providers/claude/hooks/hook_bridge/BUILD.bazel",
        "core/crates/agent/system/adapter_contract/BUILD.bazel",
        "core/crates/agent/system/authority_contract/BUILD.bazel",
        "core/crates/agent/system/agent_core/BUILD.bazel",
        "core/crates/runtime/engine/pty_custody/BUILD.bazel",
        "core/crates/runtime/engine/startup/checkpoint_contract/BUILD.bazel",
        "core/crates/process/custody_guardian/BUILD.bazel",
        "core/crates/transport/remote_broker/BUILD.bazel",
        "core/crates/transport/remote/provisioning_protocol/BUILD.bazel",
        "core/crates/distribution/bundle/source_cache/BUILD.bazel",
        "core/crates/persistence/paths/core_cache/BUILD.bazel",
        "core/crates/plugin/provider_catalog/BUILD.bazel",
    ] {
        assert_rejected(path);
    }
}

#[test]
fn every_family_rejects_flattened_roots_and_nested_packages() {
    for family in FAMILIES {
        for build in BUILD_NAMES {
            for suffix in [
                "_contract",
                "_engine",
                "_spawn",
                "_future/child",
                "/child/grand_child",
            ] {
                assert_rejected(&format!("{family}{suffix}/{build}"));
            }
        }
    }
}

#[test]
fn aliases_and_test_directories_cannot_hide_flattened_ancestry() {
    for family in FAMILIES {
        for suffix in [
            "_contract/test_support",
            "/test_support/child_contract",
            "/child_contract/unit_tests",
        ] {
            for build in BUILD_NAMES {
                let path = format!("{family}{suffix}/{build}");
                let mut file = source(&path);
                file.text = ALIAS_CONTENT.to_owned();
                let result = violations(&[file], &[]);
                assert_eq!(result.len(), 1, "{path}: {result:?}");
                assert!(result[0].message.contains("directory hierarchy"));
            }
        }
    }
}

#[test]
fn nested_packages_keep_their_independent_component_boundaries() {
    for path in [
        "core/crates/hub/client/access/store/BUILD.bazel",
        "core/crates/hub/terminal/protocol/BUILD.bazel",
        "core/crates/application/broker/supervision/state/BUILD.bazel",
        "clients/tui/client/control/protocol/BUILD.bazel",
        "build_system/architecture/checks/process/signal/retained/BUILD.bazel",
        "build_system/architecture/checks/repository/literal/BUILD.bazel",
        "core/crates/agent/providers/claude/hooks/bridge/BUILD.bazel",
        "core/crates/agent/system/hooks/runtime/BUILD.bazel",
        "core/crates/agent/contract/BUILD.bazel",
        "core/crates/agent/system/adapter/contract/BUILD.bazel",
        "core/crates/agent/system/authority/contract/BUILD.bazel",
        "core/crates/agent/system/core/BUILD.bazel",
        "core/crates/runtime/engine/pty/custody/BUILD.bazel",
        "core/crates/runtime/engine/startup/checkpoint/contract/BUILD.bazel",
        "core/crates/agent/system/control/journal/BUILD.bazel",
        "core/crates/process/spawn/BUILD.bazel",
        "core/crates/transport/remote/provisioning/protocol/BUILD.bazel",
        "core/crates/distribution/bundle/source/BUILD.bazel",
        "core/crates/persistence/paths/core/BUILD.bazel",
        "core/crates/plugin/provider/BUILD.bazel",
    ] {
        assert!(violations(&[source(path)], &[]).is_empty(), "{path}");
    }
}

#[test]
fn package_rules_do_not_reinterpret_source_files_or_test_conventions() {
    for path in [
        "core/crates/hub/client/src/transport_state.rs",
        "clients/tui/terminal/src/parser_state.rs",
        "core/crates/hub/client/test_support/BUILD.bazel",
        "core/crates/hub/client/unit_tests/BUILD.bazel",
        "core/crates/hub/client/integration_tests/BUILD.bazel",
        "build_system/architecture/test_support/BUILD.bazel",
        "core/crates/build_image/BUILD.bazel",
        "core/crates/terminal/model/BUILD.bazel",
        "core/crates/agentive_contract/BUILD.bazel",
        "core/crates/transportation_model/BUILD.bazel",
        "core/crates/applicationish_domain/BUILD.bazel",
        "other/core/crates/agent_contract/BUILD.bazel",
        "core/crates/agent_contract/src/ordinary_file.rs",
        "third_party/vendor/module_function/BUILD.bazel",
        "core/crates/hub/BUILD.bazel",
    ] {
        assert!(violations(&[source(path)], &[]).is_empty(), "{path}");
    }
}

#[test]
fn family_roots_and_exact_test_conventions_remain_valid_packages() {
    for family in FAMILIES {
        for build in BUILD_NAMES {
            for suffix in [
                "",
                "/unit_tests",
                "/integration_tests",
                "/test_support",
                "/test_support/fixture",
            ] {
                let path = format!("{family}{suffix}/{build}");
                assert!(violations(&[source(&path)], &[]).is_empty(), "{path}");
            }
        }
    }
}
