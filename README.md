# Yolo

**Persistent terminals for coding agents. Close the window. The work stays.**

[![License: MIT](https://img.shields.io/badge/license-MIT-3fb950)](LICENSE)
[![Status: Alpha](https://img.shields.io/badge/status-alpha-d29922)](docs/alpha.md)
[![Build: Bazel](https://img.shields.io/badge/build-Bazel-43a047)](MODULE.bazel)
[![Rust](https://img.shields.io/badge/rust-2024-cc6b2c)](Cargo.toml)

Yolo is a terminal workspace for people who run Claude Code, Codex and other
coding agents all day. Every managed terminal is owned by a detached Rust
Worker, so closing the TUI, restarting the Host, or reconnecting from another
client does not kill the PTY or the agent running inside it.

The first-party client is a TUI. A trusted browser client can watch and drive
the same terminals over a Hub public protocol. Neither client owns a PTY.

## Why Yolo

- **The agent is not a child of the window.** A Worker owns the PTY and its
  capability-registered process tree. The TUI and Host are replaceable.
- **Start the agent normally.** Every tab opens your shell. Yolo recognizes
  Claude Code or Codex after you launch it; creation never asks for an agent
  type.
- **Recover what can be recovered.** Client, Host and Worker failures preserve
  the live PTY when possible. After a reboot, a validated provider binding uses
  the provider's native resume command instead of pretending a dead PID is live.
- **One terminal, many views.** Writer leases serialize input. Observers can
  attach without stealing control; an explicit takeover demotes the previous
  controller.
- **Local or federated.** One Hub can expose local and remote machines; shared
  layouts and peer history remain separate from private focus and drafts.

## Architecture

```text
TUI (first-party)        Browser client / trusted plugins
        \                         /
         \-- public Hub protocol -/
                    |
                 yolo Hub
                    |
                yolo Host
                    |
          +---------+---------+
          |                   |
     session Worker      session Worker
          |                   |
         PTY                 PTY
          |                   |
    shell / Codex       shell / Claude Code
```

Rust owns terminal identity, writer leases, PTYs, output journals, process
supervision, agent detection, resume policy and recovery. The Hub owns shared
layout documents. Each client keeps its own focus, zoom and Chat drafts.

There is one Rust `yolo` executable. `yolo internal <role>` starts the Host,
Hub, relay, broker and Worker roles from the same binary. Providers are
compiled-in adapters, not plugins.

## Quickstart

Requirements: Bazelisk with the repository-pinned Bazel `8.2.1`, and a Rust
toolchain selected by Bazel. Linux needs glibc 2.31+ and cgroup v2 for
production termination. macOS platform checks need the macOS SDK.

```bash
# from a clone of this repository
bazel build --jobs=1 //core:yolo
bazel run --jobs=1 //core:yolo
```

Running `yolo` opens the TUI and starts a persistent shell. Start an agent
inside it just as you would in any terminal:

```text
Ctrl-B c   new shell tab
Ctrl-B v   split right
Ctrl-B n   next tab
Ctrl-B s   Agent runs / Hubs sidebar
Ctrl-B m   switch an agent pane between Terminal and Chat
Ctrl-B d   detach; shells and agents keep running
```

Reconnect with `yolo attach`, list terminals with `yolo list`, or open the
terminal picker with `yolo terminals`.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/AgentsMesh/YoloShell/main/scripts/install.sh | sh
```

Update an installed binary from GitHub Releases:

```bash
yolo update --check-release
yolo update --install-release
yolo update --install-release --channel nightly
```

`yolo update` without release flags checks live service admission. See
[operations](docs/operations.md) for the restart boundary.

## The Web Client

The reference plugin under `plugins/web/client` is a Bun server plus a browser
UI. It projects the Hub catalogue, live xterm.js terminals, Chat history, typed
interactions and terminal creation over an authenticated same-origin session.

```bash
yolo plugin link plugins/web/client --trust
yolo plugin startup
yolo plugin action web.client status
```

The plugin binds to the LAN and prints its URL. Open it and paste the token from
the plugin's private data directory. Plain HTTP plus a bearer token is a
trusted-LAN mode, not an internet-facing deployment. Browsers never receive the
management socket, raw Hub ID, lease grant, creation capability or Hub token.

See the [web client README](plugins/web/client/README.md) and
[ADR 0024](docs/adr/0024-web-terminal-streaming.md).

## Supported Agents

| Provider | Alpha status |
| --- | --- |
| Claude Code | Accepted target: detection, lifecycle, typed approvals and native resume |
| Codex | Accepted target: detection, lifecycle, approvals and native resume |
| Gemini CLI | Experimental adapter with hooks and native resume |
| Qwen Code | Experimental adapter with hooks and native resume |
| Grok Build | Experimental adapter with hooks and native resume |

Other terminal TUIs work as ordinary terminal programs. They do not get agent
detection, status or native resume unless a provider adapter covers them.
Yolo is independent and is not affiliated with these vendors.

## Trusted Plugins

Third-party extensions have one type: a trusted user-level plugin. A plugin has
a `yolo-plugin.json` manifest, is linked with `yolo plugin link <dir> --trust`,
and manages its own process, port and UI. Plugins are not sandboxed or approved
by the Hub; linking is the trust decision. Providers are not plugins.

Read [the trusted plugin protocol](docs/trusted-plugin-protocol.md) and
[ADR 0023](docs/adr/0023-trusted-only-plugins.md).

## Documentation

Start at the [documentation index](docs/README.md):

- [Alpha contract](docs/alpha.md) and [operations guide](docs/operations.md)
- [Architecture](docs/architecture.md),
  [concept model](docs/concept-model.md) and
  [single-executable contract](docs/single-executable-architecture.md)
- [Public JSON-RPC](docs/public-jsonrpc.md),
  [client resources](docs/client-resource-protocol.md) and
  [conversation reads](docs/conversation-reader-contract.md)
- [TUI interactions](docs/tui-interactions.md) and
  [terminal/agent lifecycle](docs/terminal-agent-model.md)
- [Architecture decision records](docs/adr/)

## Development

Bazel is the production build and test system; `Cargo.toml` is dependency
metadata only. See [CONTRIBUTING.md](CONTRIBUTING.md) for build, test,
conformance and live-acceptance commands. Every handwritten source and document
stays under 200 lines; committed build outputs and dependency directories are
forbidden.

## Project Status

Yolo is **Alpha**. The TUI supports Apple Silicon macOS 14.2+ and Linux
aarch64/x86_64. Codex and Claude Code are the accepted agent targets for this
Alpha; additional provider adapters are experimental. Linux remote deployment
remains preview. Read the [Alpha contract](docs/alpha.md) before depending on a
recovery guarantee.

## Community

- Report bugs and propose focused changes through GitHub issues.
- Read [SECURITY.md](SECURITY.md) before reporting a vulnerability.
- Participation is governed by [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

Yolo is released under the [MIT License](LICENSE). Vendored xterm.js assets and
build dependencies retain their own licenses; see
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
