# Provider extension contract

This is the Rust-side map for adding or changing an agent provider. It records
the extension seams, the two provider tiers, and the checks that keep clients,
the Worker and federation from drifting.

## Composition spine

`ProviderRuntimeRegistry` is the only compiled-in provider list
(`core/crates/agent/system/providers/src/builtin.rs`). Each provider package
returns one `ProviderRuntime`; the registry validates duplicate identities and
the port/descriptor relationships at startup.

A `ProviderRuntime` carries:

- provider descriptor: kind, install/conversation tokens, tier, capabilities,
  authority kind, lifecycle semantics, projection policy;
- adapter, prompt-submit policy, interrupt delivery, process environment keys;
- optional hook client/codec and hook install spec;
- optional conversation source (parser + evidence verification);
- optional launch command;
- interaction semantics: `none`, `terminal` or `typed`.

Conversation sources, install specs, launch commands and typed-interaction
eligibility are derived from the runtime rows. Adding a generic provider is
therefore: a provider package, one row in the builtin list, and its BUILD
dependencies.

## Two extension tiers

### Generic hook provider

Use this when the provider writes a JSON hook payload that can be decoded to
`CanonicalHookEvent`.

Required:

1. A runtime row with `AuthorityKind::GenericHookGeneration`.
2. A `HookCodec` implementing `decode`, `data_home`, profile ancestry and
   provider version; register it with `with_hook_codec`.
3. An install spec (`with_install_spec`) whose provider token matches the
   descriptor's install token.
4. A conversation source if the provider has a readable transcript
   (`with_conversation_source`).
5. A launch command (`with_launch_command`).
6. `InteractionSemantics::Terminal` (native TUI prompt) or `None`. A generic
   ingress hook is one-way: it cannot block for a user decision, so it must not
   claim `Typed`.

The generic ingress owns delivery, admission, durability, publication and
generation application. The provider owns only wire decoding and evidence
layout.

### Native authority provider

Use this when the provider needs its own authority witness, native hook server,
or a blocking decision channel (for example Claude's typed permission bridge).

This is a first-party Worker change:

1. Name the authority in `AuthorityKind` and add the Worker dispatch arm in
   `core/crates/runtime/engine/worker/src/native_control/source.rs`.
2. Implement native admission, the hook connection and its lifecycle.
3. Register pending interactions through the provider-neutral
   `native_control::interaction` store with the agent `ProcessIdentity`.
4. Declare `InteractionSemantics::Typed` only when the native bridge can return
   a durable decision. The registry rejects `Typed` without the bridge
   authority; currently only `ClaudeGeneration` is bridged.
5. Add the provider to the typed-interaction conformance test.

Do not add a dynamic provider ABI. Providers are trusted compiled-in code; a
native authority bridge must remain a reviewable Worker change.

## Conversation projection

`TranscriptParser` returns `ParsedRecord` with an explicit disposition:

- an `entry` is a projected turn;
- `ignored` is a known provider-internal record that must not make history
  unavailable;
- `unsupported` is content the reader could not project and may raise
  `UnsupportedRecord` in the page's partial reasons.

Providers should classify known metadata as `ignored` instead of leaving it to
the catch-all. Claude's `attachment`, `api-request*`, mode and cost records are
ignored this way; Codex's injected developer/environment context is ignored.
Unknown future content still stays `unsupported`, so real gaps remain visible.

## Shared state projection

Lifecycle-to-`AgentState` derivation lives in
`yolo_agent_hook_generation::project` with `ProjectionRules::GENERATION` and
`ProjectionRules::CODEX`. Provider-specific projections must use the shared
rules and declare only differences (authoritative instrumentation, final-hook
exit, Stop candidate, unknown-turn handling). Provider code must not repeat the
attention mapping; that duplication previously let Codex diverge on permission
candidates.

## Public provider capabilities

`list_providers` returns `ProviderFeatures` with conversation, prompt,
interrupt and interaction semantics. Fields are serde-defaulted so a new Hub
reads a reply from a peer that predates them. `ProviderSummary` keeps its
strict wire, so the reverse direction (a peer that predates the field reading a
new reply) needs the fleet roll, not a compatibility shim. Clients use the
features to choose Chat/terminal affordances without probing a live session. The Web provider picker validates and forwards them;
the Needs-you list uses `interaction = typed` to wait for a typed card and
otherwise offers a terminal-native approval.

## Federation and origin services

Peer-origin calls bypass client access but still need origin-side decorators.
Every method that reads Host state is listed in
`ORIGIN_SERVICE_METHODS`; the history and provider decorators declare their
`ORIGIN_METHODS`, and the coverage test requires the union to equal the table.
A new federated service method updates the protocol table, the decorator
method list and the decorator composition together.

## Checks

- `//core/crates/agent/system/providers:yolo_agent_providers_unit_tests`:
  runtime/install/codec/conversation/launch/interaction composition.
- `//core/crates/agent/system/provider/runtime:unit_tests`: typed semantics
  require the native bridge authority.
- `//core/crates/conversation/transcript:unit_tests`: ignored vs unsupported.
- Provider parser tests for Claude and Codex.
- `//core/crates/runtime/engine/worker:yolo_worker_unit_runtime_tests`
  (`codex_hook`): shared projection behavior.
- `//core/crates/hub/local/providers:unit_tests`: origin method coverage.
- `//build_system/architecture:conformance_test`.
