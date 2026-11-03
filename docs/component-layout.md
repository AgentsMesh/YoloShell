# Component directory ownership

Component families use directories to express their domain and responsibility.
Each leaf component owns a Bazel package, a Rust library, and its separate
`src`, `unit_tests`, `integration_tests`, and `test_support` directories when
needed. A directory can contain both its own crate and independent child crates;
Bazel stops a parent package's glob at each child `BUILD.bazel` boundary.

Examples of the current ownership structure:

| Family | Components |
| --- | --- |
| Hub agent | `core/crates/hub/agent/{projection,protocol}` |
| Hub client access | `core/crates/hub/client/access/{dispatch,protocol,registry,runtime,store}` |
| Hub remote setup | `core/crates/hub/remote/setup/{protocol,runtime,store}` |
| Hub terminal | `core/crates/hub/terminal/{client,io,relay}` |
| Application broker | `core/crates/application/broker/{child/custody,profile/authority,startup,supervision}` |
| Conversation | `core/crates/conversation/{protocol,service,source,transcript}` |
| Presentation | `core/crates/presentation/{layout,protocol,store}` |
| Terminal metadata | `core/crates/terminal/metadata/{protocol}` |
| Agent hook contracts | `core/crates/agent/hooks` |
| Agent hook runtime | `core/crates/agent/system/hooks/{codec,ingress,installation,runtime,lineage}` |
| Agent hook authority | `core/crates/agent/system/authority/generation` |
| Provider hooks | `core/crates/agent/providers/{claude,codex,gemini,grok,qwen}/hooks` |
| Provider policy packages | `core/crates/agent/providers/{claude,codex,gemini,grok,qwen}/provider` (identity and policy targets) |
| Provider adapters and parsers | `core/crates/agent/providers/{claude,codex,gemini,grok,qwen}` (adapter crate) and `.../<provider>/parser` |
| Agent providers (only provider code) | `core/crates/agent/providers` |
| Agent control | `core/crates/agent/system/control/{contract,journal,runtime}` |
| Agent contracts and runtime | `core/crates/agent/{contract,runtime}` |
| Agent adapters and foundation | `core/crates/agent/system/{adapter/contract,adapter/registry,authority/contract,core}` |

`core/crates/agent/providers` contains only the five provider trees; each
agent's adapter, hooks, parser, identity and policy packages sit below its own
directory. The runtime composition root remains mechanism at
`core/crates/agent/system/providers`.
| Shared process | `core/crates/process/spawn` |
| Runtime engine | `core/crates/runtime/engine/{executable/identity,process/identity,pty/custody,startup,worker,host}` |
| Distribution | `core/crates/distribution/{bundle/source,publication/protocol,release/store}` |
| Persistence | `core/crates/persistence/{coordination/core,durable/reboot,maintenance/contract,paths/core}` |
| Transport | `core/crates/transport/{broker,local/unix,protocol/validation,remote,ssh}` |
| TUI creation | `plugins/tui/creation/{journal,recovery}` |
| TUI client control | `plugins/tui/client/control` and its `protocol` component |
| TUI tab state | `plugins/tui/tab` and its `manager` component |
| Process signal checks | `build_system/architecture/process/signal` |
| Runtime safety checks | `build_system/architecture/runtime/safety` |
| Repository policies | `build_system/architecture/repository/policy` |
| Swift process checks | `build_system/architecture/swift/process` |

Every first-party domain family has a root Bazel package that owns its
`architecture_check_data` aggregation. Leaf crates remain below that root and
own their production and test targets. Current domain roots include
`agent`, `application`, `conversation`, `distribution`, `hub`, `persistence`,
`plugin`, `presentation`, `process`, `runtime`, `terminal`, `transport`, and
`tui`.

Directories determine ownership. Rust crate identifiers remain globally unique,
such as `yolo_hub_client_access_store`, because independently compiled Rust
libraries still need unambiguous dependency names. Those identifiers are neither
filesystem aliases nor compatibility paths. Dependency labels point directly at
the current directory; retired flat package paths have no forwarding targets.

The repository architecture policy inspects Bazel package locations. It rejects
flattened component names in the Agent, Hub, application, Conversation,
Presentation, Terminal, TUI, process, runtime, distribution, persistence,
transport, plugin, and architecture families,
including flattening introduced below an otherwise correct parent directory.
Conventional test directory names and Rust source filenames retain their normal
naming. The retired flat Agent roots are rejected explicitly, so a removed
package cannot return as an alias or compatibility target.

Other domain layouts are governed by their existing ownership rules. The
hierarchy policy deliberately scopes its enforcement to component families with
an observed domain and responsibility axis; compound domain names do not imply a
universal underscore-to-directory rewrite.
