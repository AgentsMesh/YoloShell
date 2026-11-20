"""Graph-only fixtures: genquery inspects these edges without building targets."""

load(
    "//build_system/architecture:topology.bzl",
    "YOLO_PROCESS",
)

def _graph(_ctx):
    return []

dependency_graph = rule(
    implementation = _graph,
    attrs = {
        "deps": attr.label_list(),
        "data": attr.label_list(),
        "tool": attr.label(cfg = "exec"),
    },
)

# The runtime boundary must preserve implicit target dependencies; using
# --noimplicit_deps to hide a schema compiler would also hide this violation.
implicit_runtime_graph = rule(
    implementation = _graph,
    attrs = {
        "_runtime": attr.label(default = YOLO_PROCESS),
    },
)
