# Repository review skills

`agents/skills/` owns the shared review workflows and their bundled references:

- `arch-check`: concept modeling, responsibilities, invariants, dependencies,
  modularity and change history; supports a report, `--plan` and `--fix`.
- `code-review`: independent correctness and cleanup reviews with verification;
  supports effort levels, `--fix` and explicit `--comment` for PR/MR targets.

These skills were copied from an internal working tree's working tree on 2026-10-09 (source HEAD
`e1e1d0e0ae26c1aeb7dbfad579b6915f52b4ab71`). All nine reference documents were
included. Project-specific rules and validation instructions were adapted to
Yolo's Rust/Bazel, macOS, Ubuntu, PTY and communication architecture. The source
repository was not modified.

Codex discovers the shared skills through relative links in `.agents/skills/`.
Claude Code uses thin adapters in `.claude/skills/`; their `shared` links point
to the same canonical workflows. Edit the shared source once, keeping client
invocation details in the adapters. Links must remain relative and resolve
inside this repository.

Repository-relative paths in review instructions start at the checkout root;
bundled `references/` paths start at the canonical skill directory. Existing
user instructions and `AGENTS.md` govern the requested scope and authorization.
Copying these skills does not itself run a review or publish review comments.

Import validation: both skill names, all nine references, and all four relative
discovery/shared links were checked. All 14 added Markdown files are below
200 lines; whitespace validation and an independent path/content review passed.
No product runtime or service was changed by this import.
