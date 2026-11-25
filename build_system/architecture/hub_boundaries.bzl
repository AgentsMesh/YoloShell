"""Public Hub models cannot inherit local runtime or controller dependencies."""

load(
    "//build_system/architecture:topology.bzl",
    "CAPNP_ARGUMENT",
    "CAPNP_SCHEMA",
)

load(":hub_roles.bzl", "CLIENT_COMPONENTS", "HUB_ENTRY_COMPONENTS", "PUBLIC_COMPONENTS", "RUNTIME_COMPONENTS")
load(":runtime_dependencies.bzl", "RUNTIME_QUERY_OPTIONS")

# Exact public model packages plus their pure cryptographic values. Derive the
# paths from the checked roots so a directory move cannot leave a stale regex.
_ALLOWED_PACKAGES = "^({}):".format("|".join(
    [component.split(":")[0] for component in PUBLIC_COMPONENTS] + [
        "//core/crates/crypto",
        "//core/crates/transport/capnp/value",
        "//core/crates/transport/capnp/codec",
        CAPNP_ARGUMENT,
        CAPNP_SCHEMA,
    ],
))

_CLIENT_FORBIDDEN = "^//core/crates/(runtime/engine|application|agent/system|persistence|terminal/model)(/|:)"

# Pure std-only executable/argv values do not grant local runtime access.
_CLIENT_LAUNCH_CONTRACT = "^//core/crates/application/launch/contract:"

def public_forbidden_dependencies(components):
    """Public models may reach only exact public-model packages."""
    dependencies = "deps(set({}))".format(" ".join(components))
    return "(filter('^//', {}) except filter('{}', {}))".format(
        dependencies,
        _ALLOWED_PACKAGES,
        dependencies,
    )

def hub_package_dependency_test():
    """All Hub packages, not only public/client, stay below role entries."""
    roots = [
        component
        for component in PUBLIC_COMPONENTS + CLIENT_COMPONENTS + RUNTIME_COMPONENTS
        if component.startswith("//core/crates/hub/")
    ]
    duals = [component for component in roots if component in HUB_ENTRY_COMPONENTS]
    cases = [([component for component in roots if component not in duals], HUB_ENTRY_COMPONENTS)]
    for dual in duals:
        cases.append(([dual], [entry for entry in HUB_ENTRY_COMPONENTS if entry != dual]))
    tests = []
    for index, (case_roots, entries) in enumerate(cases):
        suffix = "" if index == 0 else "_dual_{}".format(index - 1)
        query = "hub_entry_dependencies" + suffix
        test = "hub_package_dependency" + suffix + "_test"
        native.genquery(
            name = query,
            expression = "rdeps(deps(set({})), set({}))".format(
                " ".join(case_roots),
                " ".join(entries),
            ),
            opts = RUNTIME_QUERY_OPTIONS,
            scope = case_roots + entries,
        )
        native.sh_test(
            name = test,
            srcs = ["integration_tests/hub_package_dependencies.sh"],
            args = ["$(location :{})".format(query)],
            data = [":{}".format(query)],
        )
        tests.append(":{}".format(test))
    # A dual-role root is checked against every entry but itself.
    native.test_suite(name = "hub_package_dependency_tests", tests = tests)

def hub_dependency_boundary_test():
    clients = "deps(set({}))".format(" ".join(CLIENT_COMPONENTS))
    native.genquery(
        name = "hub_forbidden_dependencies",
        expression = "{} union (filter('{}', {}) except filter('{}', {}))".format(
            public_forbidden_dependencies(PUBLIC_COMPONENTS),
            _CLIENT_FORBIDDEN,
            clients,
            _CLIENT_LAUNCH_CONTRACT,
            clients,
        ),
        opts = RUNTIME_QUERY_OPTIONS,
        scope = PUBLIC_COMPONENTS + CLIENT_COMPONENTS,
    )
    native.sh_test(
        name = "hub_dependency_boundary_test",
        srcs = ["integration_tests/hub_dependency_boundary.sh"],
        args = ["$(location :hub_forbidden_dependencies)"],
        data = [":hub_forbidden_dependencies"],
    )
