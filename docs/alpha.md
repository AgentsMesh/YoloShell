# Yolo 0.1 Alpha Contract

This document defines what the first public Alpha claims and what it does not.
Alpha means the boundaries are usable and tested, not that every interface is
frozen. Read the limits before relying on a recovery or compatibility promise.

## Current state

The TUI, local Hub/Host runtime, provider adapters and trusted plugin interface
are usable for daily terminal work on the supported platforms. The web client is
a reference plugin in
[AgentsMesh/YoloShellWeb](https://github.com/AgentsMesh/YoloShellWeb). Protocol
and lifecycle contracts may still change with a versioned migration.

## Support matrix

| Surface | Alpha commitment |
| --- | --- |
| TUI client | Apple Silicon macOS 14.2 or newer, and Linux aarch64 or x86_64. |
| Local runtime | Login shells, Codex CLI, Claude Code, and generic terminal TUIs. |
| Remote runtime | Apple Silicon macOS over direct SSH; Linux remote is preview. |
| Codex CLI | Accepted target for detection, lifecycle, approvals and native resume. |
| Claude Code | Accepted target for detection, lifecycle, typed interactions and native resume. |
| Gemini CLI | Experimental adapter with hooks and native resume. |
| Qwen Code | Experimental adapter with hooks and native resume. |
| Grok Build | Experimental adapter with hooks and native resume. |
| Agent status | Authenticated provider lifecycle hooks are authoritative; PTY or transcript guesses do not upgrade status. |
| Other agent TUIs | Terminal-only; no detection, status or native resume guarantee. |

Provider versions outside the accepted targets are not covered by the Alpha
compatibility claim until the same parser and live workflow pass. Yolo is
independent of the provider vendors.

## Included workflows

- Every new tab starts the configured host's shell. Agent type is never chosen
  during tab creation.
- Codex and Claude Code are recognized after the user launches them from a
  shell. A resumable binding is published only after the provider hook handshake
  and evidence are validated.
- Running, approval, input, compaction, failure and exit states come from
  authenticated provider lifecycle events. Hook-disabled, untrusted, bypassed or
  incomplete instrumentation remains visibly unmanaged and `Unknown`.
- Recursive row and column splits, ratios, focus, zoom, swaps and session
  bindings persist across TUI launches.
- Closing a tab or detaching the TUI removes the view while the Worker, PTY and
  process tree remain alive.
- Client or Host replacement reattaches without replacing a healthy Worker, PTY
  or child process.
- A machine reboot creates a new runtime epoch and uses provider-native resume
  only for an unambiguous binding; plain shells require explicit recreation.
- One SSH transport multiplexes runtime, attachment, bounded command, SFTP read
  and deployment channels. A transport reset preserves the remote session.
- Writer ownership is explicit and fenced. A newly created shell begins
  writable without showing `Read Only` or requiring Take Control.
- The trusted web client in `YoloShellWeb` projects the catalogue, terminal
  streams, Chat history, typed interactions and terminal creation through the
  public Hub protocol.

## Deferred from Alpha

- Linux remote deployment and disconnect recovery remain preview.
- Password authentication, keyboard-interactive authentication, ProxyJump and
  OpenSSH configuration expansion are unsupported.
- Third-party plugin distribution, signing, marketplace trust and sandbox
  parity are not implemented. Plugins remain trusted user-level processes.
- Structured provider APIs, collaborative multi-writer terminals and additional
  first-party clients are outside the product boundary.
- The web client plugin in `YoloShellWeb` is a reference client, not an
  internet-facing service. Plain HTTP plus a bearer token is limited to trusted
  LANs.

## Verification approach

A release candidate is evaluated against one source revision:

1. Rust unit, integration and runtime-resilience suites.
2. Bazel architecture and conformance gates, including source and dependency
   boundaries.
3. Private-PTY TUI acceptance that runs serially and does not change window
   focus.
4. Real Codex and Claude Code workflows: launch from a shell, detect, observe
   approvals and input, replace the client and Host, then resume natively after
   a reboot.
5. Remote macOS transport deployment and recovery over SSH.
6. The Bun web client in `YoloShellWeb`: link with trust, project terminal and
   Chat state, answer a typed interaction, create a terminal, exercise two-tab
   control, restart the plugin and reconnect.

A passing unit suite is not production approval. Evidence is recorded with the
source revision used for the run.

## Compatibility policy

- Wire changes that affect public clients are versioned and migrate all clients
  together.
- Unreleased contracts may change; see
  [the unreleased contract policy](unreleased-contract-policy.md).
- Recovery semantics are provider-specific and are never inferred from terminal
  bytes alone.
- A new provider adapter starts experimental until its detection, lifecycle and
  native resume pass the same live workflow as the accepted targets.
