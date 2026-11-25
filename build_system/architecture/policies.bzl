"""Single registration manifest for architecture policy crates."""

POLICIES = [
    struct(component = "//build_system/architecture/checks/claude", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/graph", library = False, test_dependency = "//build_system/architecture/graph:library", tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/cargo", library = True, test_dependency = "//build_system/architecture/checks/cargo:test_support", tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/foundation", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/hub", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/contracts", library = False, test_dependency = "//build_system/architecture/contracts:library", tests = []),
    struct(component = "//build_system/architecture/checks/process/signal/catalog", library = False, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/process/signal/child", library = False, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/process/signal/foundation", library = False, test_dependency = "//build_system/architecture/checks/process/signal/foundation:library", tests = [":internal_unit_tests", ":unit_tests"]),
    struct(component = "//build_system/architecture/checks/process/signal/path", library = False, test_dependency = "//build_system/architecture/checks/process/signal/path:library", tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/process/signal/policy", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/process/signal/retained", library = False, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/repository/rules", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/repository", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/runtime/safety/policy", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/source", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/syntax", library = False, test_dependency = "//build_system/architecture/syntax:library", tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/dependencies/usage", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/entrypoint", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/checks/ownership", library = True, test_dependency = None, tests = [":unit_tests"]),
    struct(component = "//build_system/architecture/test_support", library = False, test_dependency = "//build_system/architecture/test_support:library", tests = []),
]

POLICY_CHECK_DATA = [policy.component + ":architecture_check_data" for policy in POLICIES]

POLICY_LIBRARIES = [policy.component + ":library" for policy in POLICIES if policy.library]

POLICY_TEST_DEPENDENCIES = [policy.test_dependency for policy in POLICIES if policy.test_dependency]

POLICY_TEST_TARGETS = [policy.component + test for policy in POLICIES for test in policy.tests]

POLICY_DISPATCHES = [
    ("//build_system/architecture/checks/claude", "inspect_claude_policy"),
    ("//build_system/architecture/checks/cargo", "cargo_rules"),
    ("//build_system/architecture/checks/hub", "inspect_hub_policy"),
    ("//build_system/architecture/checks/process/signal/policy", "inspect_process_signal_policy"),
    ("//build_system/architecture/checks/process/signal/policy", "inspect_process_signal_repository"),
    ("//build_system/architecture/checks/repository/rules", "repository_rules"),
    ("//build_system/architecture/checks/repository", "inspect_repository_policy"),
    ("//build_system/architecture/checks/repository", "inspect_repository_contracts"),
    ("//build_system/architecture/checks/runtime/safety/policy", "inspect_runtime_safety"),
    ("//build_system/architecture/checks/runtime/safety/policy", "inspect_runtime_safety_repository"),
    ("//build_system/architecture/checks/source", "brand_rules"),
    ("//build_system/architecture/checks/source", "line_rules"),
    ("//build_system/architecture/checks/source", "source_policy"),
    ("//build_system/architecture/checks/dependencies/usage", "inspect_dependency_usage"),
    ("//build_system/architecture/checks/entrypoint", "inspect_target_entrypoint"),
    ("//build_system/architecture/checks/ownership", "inspect_target_ownership"),
]

_COMPONENTS = [policy.component for policy in POLICIES]


def _validate_dispatches(dispatches, components):
    unknown = [dispatch[0] for dispatch in dispatches if dispatch[0] not in components]
    if unknown:
        fail("policy dispatch names an unregistered component: " + unknown[0])


_validate_dispatches(POLICY_DISPATCHES, _COMPONENTS)
