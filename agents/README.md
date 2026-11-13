# Repository agent skills

`agents/skills/` owns the shared workflows and their bundled references or
scripts:

- `arch-check`: concept modeling, responsibilities, invariants, dependencies,
  modularity and change history; supports a report, `--plan` and `--fix`.
- `code-review`: independent correctness and cleanup reviews with verification;
  supports effort levels, `--fix` and explicit `--comment` for PR/MR targets.
- `merge`: commits, rebases, pushes, creates or reuses a GitHub pull request,
  monitors GitHub Actions checks and merges it; supports squash, merge and rebase
  and explicit `--no-merge` / `--no-monitor` modes.

The review skills were imported on 2026-10-09 (source HEAD
`e1e1d0e0ae26c1aeb7dbfad579b6915f52b4ab71`); all nine review references were
included. The merge skill was adapted on 2026-11-03 from the internal merge
workflow, replacing GitLab `glab` with GitHub `gh`, PR checks and protected
merge rules. Project-specific rules were adapted to Yolo's Rust/Bazel, macOS,
Ubuntu, PTY and communication architecture.

Codex discovers the shared skills through relative links in `.agents/skills/`.
Claude Code uses thin adapters in `.claude/skills/`; their `shared` links point
to the same canonical workflows. Edit the shared source once, keeping client
invocation details in the adapters. Links must remain relative and resolve
inside this repository.

Repository-relative paths in review instructions start at the checkout root;
bundled `references/` paths start at the canonical skill directory, and merge
scripts start at the merge skill directory. Existing user instructions and
`AGENTS.md` govern the requested scope and authorization. Copying these skills
does not itself run a review, create a PR or merge anything.

Import validation: all three skill names, nine review references, six relative
discovery/shared links, and the merge shell/Python helpers were checked. Every
added Markdown, shell and Python file is below 200 lines; whitespace validation
and independent path/content review passed. No product runtime or service was
changed by these imports.
