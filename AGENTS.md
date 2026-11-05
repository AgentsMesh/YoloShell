# Development guide

Yolo is a Rust repository built with Bazel. The TUI is the first-party client in
`clients/tui`. The trusted web client and its TypeScript SDK live in
`AgentsMesh/YoloShellWeb`; the plugin registry lives in
`AgentsMesh/YoloShellPlugins`.

## Environment

- Use Bazelisk with the pinned Bazel version. Bazel defines every production
  target; Cargo only supplies dependency metadata.
- Keep platform-specific checks on their platform. The first-party client is
  the TUI; there is no AppKit or Swift client.
- Run automated terminal acceptance in private PTYs and never change window
  focus.
- Preserve live PTYs when testing recovery. Kill exact PIDs, never process
  names shared by workers.
- Keep generated build outputs outside the source tree.

## Changes

- Edit source in this checkout. Commit completed changes with the relevant
  validation evidence.
- Keep every handwritten source and document below 200 lines. Split by
  responsibility rather than adding a numbered part file.
- Treat the Bazel architecture and conformance gates as authoritative. Do not
  weaken a gate to land a change.
- Keep public browser payloads free of the management socket, Hub ID, lease
  grants, creation capabilities and plugin tokens.
- Preserve existing wire contracts unless a versioned migration changes all
  clients together.

## Review

- [arch-check](agents/skills/arch-check/SKILL.md): architecture, concept and
  responsibility review with evidence and independent verification.
- [code-review](agents/skills/code-review/SKILL.md): diff review with
  independent finding and verification stages.

Read the selected skill before invoking its workflow. Shared sources and client
discovery entries are described in [agents/README.md](agents/README.md).

Internal maintainers should also read `AGENTS.private.md` when present. That
file is not part of the public export.
