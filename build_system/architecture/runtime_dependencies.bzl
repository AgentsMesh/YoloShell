"""Runtime boundaries follow target edges without traversing execution tools."""

# Exclude edges with cfg = "exec", not the targets they happen to reach.
# A tool also named in ordinary deps/data remains visible, as do implicit
# dependencies in the target configuration. Never subtract a tool's deps set.
RUNTIME_QUERY_OPTIONS = ["--notool_deps"]
