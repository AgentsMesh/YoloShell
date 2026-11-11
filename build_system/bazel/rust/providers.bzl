"""Single compiled-in provider table.

Adding a provider edits one row here plus its directory under
`core/crates/agent/providers`. The runtime component lists, worker dependency
list, component test graph, runtime test suite, client resource tests, hook
boundary set and architecture inventories all derive from this table.
`topology.bzl` keeps explicit export constants; `providers_table_test` checks
that every label used here is exported there.

`runtime_components` mirrors the core facade: a label belongs there only when
`//core:yolo_runtime` re-exports it. `worker_components` mirrors the Worker's
compiled-in dependency list. Keep both explicit per provider; the two lists
intentionally differ today.
"""

load(":provider_rows/claude.bzl", "CLAUDE")
load(":provider_rows/codex.bzl", "CODEX")
load(":provider_rows/gemini.bzl", "GEMINI")
load(":provider_rows/grok.bzl", "GROK")
load(":provider_rows/qwen.bzl", "QWEN")

PROVIDERS = [
    CLAUDE,
    CODEX,
    GEMINI,
    GROK,
    QWEN,
]

def _flatten(lists):
    result = []
    for items in lists:
        result.extend(items)
    return result

PROVIDER_RUNTIME_COMPONENTS = _flatten([p.runtime_components for p in PROVIDERS])
PROVIDER_WORKER_COMPONENTS = _flatten([p.worker_components for p in PROVIDERS])
PROVIDER_RUNTIME_TESTS = _flatten([p.runtime_tests for p in PROVIDERS])
PROVIDER_CLIENT_TESTS = _flatten([p.client_tests for p in PROVIDERS])
PROVIDER_HOOK_BOUNDARY_COMPONENTS = _flatten([p.hook_boundary for p in PROVIDERS])
PROVIDER_AGENT_ARCHITECTURE = _flatten([p.architecture_agent for p in PROVIDERS])
PROVIDER_CONVERSATION_ARCHITECTURE = _flatten([p.architecture_conversation for p in PROVIDERS])
PROVIDER_CRATE_DEPS = _flatten([p.provider_crate_deps for p in PROVIDERS])
PROVIDER_COMPONENT_ENTRIES = _flatten([p.component_entries for p in PROVIDERS])
PROVIDER_TOPOLOGY_LABELS = _flatten([p.topology_labels for p in PROVIDERS])
