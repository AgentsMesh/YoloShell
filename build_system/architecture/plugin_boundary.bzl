"""Enforce the plugin boundary on Bazel's configured dependency graph."""

_CORE_PREFIX = "core/crates/"
_COMPOSITION = "core/crates/application/yolo"
_PLUGIN_PREFIX = "plugins/"
_UI_CRATES = ["ratatui", "crossterm"]
_Boundary = provider(fields = {"ui_dependencies": "Transitive implementation labels"})

def _is_ui(label):
    return label.package.startswith(_PLUGIN_PREFIX) or (
        label.repo_name and
        any([label.name == name or label.name.startswith(name + "_") for name in _UI_CRATES])
    )

def _boundary_impl(target, ctx):
    inherited = []
    for dependency in getattr(ctx.rule.attr, "deps", []):
        if _Boundary in dependency:
            inherited.append(dependency[_Boundary].ui_dependencies)
    dependencies = depset(
        [str(target.label)] if _is_ui(target.label) else [],
        transitive = inherited,
    )
    if (target.label.package.startswith(_CORE_PREFIX) and
        target.label.package != _COMPOSITION and
        not getattr(ctx.rule.attr, "testonly", False)):
        forbidden = dependencies.to_list()
        if forbidden:
            fail("core cannot depend on client implementations: {} -> {}".format(target.label, forbidden))
    return [_Boundary(ui_dependencies = dependencies)]

_plugin_boundary = aspect(implementation = _boundary_impl, attr_aspects = ["deps"])

def _check_impl(ctx):
    output = ctx.actions.declare_file(ctx.label.name + ".txt")
    ctx.actions.write(output, "Configured core/plugin dependency boundary passed.\n")
    return [DefaultInfo(files = depset([output]))]

plugin_boundary_check = rule(
    implementation = _check_impl,
    attrs = {"roots": attr.label_list(aspects = [_plugin_boundary])},
)
