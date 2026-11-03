# Third-party notices

Yolo is MIT licensed. This file summarizes third-party material that is
distributed in this repository or fetched by the build.

## Vendored browser assets

`plugins/web/client/vendor/` contains pinned xterm.js and addon-fit assets used
by the trusted web client. Their MIT license texts and SHA-256 manifest are in
that directory. They are the only vendored browser source in the repository.

## Build dependencies

Rust crates are declared in `Cargo.toml` and pinned in `Cargo.lock`. Bazel
module dependencies are declared in `MODULE.bazel` and pinned in
`MODULE.bazel.lock`. Build-system patches under `third_party/` retain their
upstream notices where applicable.

## Product names

Claude Code, Codex, Gemini, Qwen and Grok are third-party products. Yolo is an
independent terminal supervisor and is not affiliated with or endorsed by their
vendors. Product names are used only to describe provider integrations.

## Skills

The architecture and diff review workflows under `agents/` are development
tooling, not part of the Yolo runtime. See `agents/README.md` for their
provenance and the client discovery entries under `.agents/` and `.claude/`.
