"""Enforce the client boundary on Bazel's configured dependency graph."""

_CORE_PREFIX = "core/crates/"
_COMPOSITION = "core/crates/application/yolo"
_CLIENT_PREFIXES = ["clients/"]
_UI_CRATES = ["ratatui", "crossterm"]
_Boundary = provider(fields = {"client_dependencies": "Transitive implementation labels"})

def _is_client(label):
    return any([label.package.startswith(prefix) for prefix in _CLIENT_PREFIXES]) or (
        label.repo_name and
        any([label.name == name or label.name.startswith(name + "_") for name in _UI_CRATES])
    )

def _boundary_impl(target, ctx):
    inherited = []
    for dependency in getattr(ctx.rule.attr, "deps", []):
        if _Boundary in dependency:
            inherited.append(dependency[_Boundary].client_dependencies)
    dependencies = depset(
        [str(target.label)] if _is_client(target.label) else [],
        transitive = inherited,
    )
    if (target.label.package.startswith(_CORE_PREFIX) and
        target.label.package != _COMPOSITION and
        not getattr(ctx.rule.attr, "testonly", False)):
        forbidden = dependencies.to_list()
        if forbidden:
            fail("core cannot depend on client implementations: {} -> {}".format(target.label, forbidden))
    return [_Boundary(client_dependencies = dependencies)]

_client_boundary = aspect(implementation = _boundary_impl, attr_aspects = ["deps"])

def _check_impl(ctx):
    output = ctx.actions.declare_file(ctx.label.name + ".txt")
    ctx.actions.write(output, "Configured core/client dependency boundary passed.\n")
    return [DefaultInfo(files = depset([output]))]

client_boundary_check = rule(
    implementation = _check_impl,
    attrs = {"roots": attr.label_list(aspects = [_client_boundary])},
)
