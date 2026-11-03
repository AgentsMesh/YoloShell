# Release pipeline

Releases are orchestrated by GitLab and built by GitHub Actions, so Linux
artifacts compile on Linux runners and the macOS artifact compiles on a macOS
runner.

## Channels

- **Stable**: push a tag such as `v0.1.0`. GitLab syncs the sanitized public
  snapshot, dispatches the GitHub release workflow, and waits for its result.
- **Nightly**: a daily GitLab schedule builds the current `main` revision and
  dispatches a prerelease tagged `nightly-<date>-<sha>`.

## Assets

```text
yolo-<version>-aarch64-apple-darwin.tar.gz
yolo-<version>-x86_64-unknown-linux-gnu.tar.gz
yolo-<version>-aarch64-unknown-linux-gnu.tar.gz
SHA256SUMS
```

Linux archives are built in pinned Debian bullseye containers (glibc 2.31) on
GitHub's Ubuntu amd64 and arm64 runners. The macOS archive is built natively on
the Apple Silicon `macos-15` runner.

## GitLab stage

1. `sync-public` exports the sanitized snapshot and force-pushes the public
   repository's single `main` commit.
2. `trigger-release` dispatches `yolo-release` to GitHub Actions with the
   release tag, version, name, prerelease flag and source commit, then polls
   the workflow until it completes.

The GitLab runner only needs `git`, `curl`, `perl` and `python3`; it does not
need a compiler or Docker.

## GitHub Actions workflow

1. `verify` runs the architecture conformance gate on Ubuntu.
2. `build-linux-amd64` builds in the pinned Linux amd64 container.
3. `build-linux-arm64` builds on the native arm64 Ubuntu runner.
4. `build-macos-arm64` builds on `macos-15`.
5. `publish` downloads all release archives, regenerates `SHA256SUMS`, creates
   the GitHub Release and uploads every asset.

Build jobs rewrite `build_system/bazel/version.bzl` in their own checkout so
the binary reports the release version. That generated change is never pushed.

## Required settings

Create a masked, protected GitLab CI/CD variable named `GITHUB_TOKEN` with
permission to write repository contents and Actions workflows for
`AgentsMesh/YoloShell`. It is used to push the public snapshot and dispatch the
release workflow. GitHub Actions uses its own `GITHUB_TOKEN` to publish the
release.

The public repository must keep the `Release` workflow on `main`. GitLab
force-pushes a fresh sanitized snapshot before every release; a site change in
that snapshot also deploys the website.

## Nightly schedule

A GitLab pipeline schedule on the default branch runs daily. The GitLab
pipeline computes `0.0.0-nightly.<date>.<sha>` and dispatches the GitHub
prerelease. The workflow only runs for `v*` tags, scheduled pipelines and
manually started web pipelines; an ordinary branch push does not publish.

## Installer and updater

The release body and README point at:

```bash
curl -fsSL https://raw.githubusercontent.com/AgentsMesh/YoloShell/main/scripts/install.sh | sh
```

`yolo update --check-release` reads the GitHub release API;
`yolo update --install-release` downloads the matching archive, verifies
`SHA256SUMS`, installs an immutable image under `~/.local/lib/yolo/<sha>/yolo`
and atomically updates `~/.local/bin/yolo`. `--channel nightly` selects the
newest prerelease. The updater never restarts Host/Hub services; run
`yolo update` for the existing live-service admission check before restarting
them.
