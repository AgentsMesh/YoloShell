# Local Service Lifecycle

Yolo runs one local Host and Hub for the current profile. The lifecycle commands
inspect and control them without entering the TUI. See
[upgrade admission](upgrade-admission.md) for the compatibility boundary that
`yolo update` and `yolo restart` enforce.

## Status and Version

```bash
yolo status          # Hub/Host PIDs, images, protocol and admission state
yolo version         # installed image and the image each service is running
yolo start           # start the local Host and Hub
yolo stop            # stop the local Hub, then the Host
yolo restart         # stop and start the same profile
```

All five commands address the profile selected by `--local-profile` or the
platform default, and accept `--json`. They are local-only: `--hub`, `--socket`
and `--endpoint` describe a remote entry and are rejected. `yolo --version`
remains the single-line binary version contract that remote deployment probes
compare byte for byte, so it never reports service state; use `yolo version`
when the question is which image is installed versus which image the running
Host and Hub actually mapped.

## Stop and Restart

`yolo stop` resolves each service PID from its private Unix socket's peer
credentials and sends SIGTERM to that exact PID; it never matches process names.
It stops the Hub first and then the Host. Host shutdown closes ingress but leaves
session Workers attached to their PTYs, so stopping services does not terminate
running shells; a later `yolo start` reconnects the Hub to the existing Host and
Workers. If a service does not exit within the bound, the command fails with the
exact PID instead of escalating to SIGKILL.

`yolo restart` validates the runtime contract and, on Linux, delegated Host
custody before stopping anything, so an incompatible state or an un-delegated
shell leaves the running services untouched.

On Linux a new Host requires delegated cgroup v2 custody. `yolo start` reports
that requirement before launching anything; start the profile under a delegated
systemd service, or attach to an existing Hub instead.

If a Host from another image is already running, `yolo start` leaves it intact
and reports the new Hub's exit, its error and the running Host image; restart
the profile with `yolo restart` to replace both services.

## GitHub Releases and Updates

Official assets are published to GitHub Releases by the GitHub Actions release
workflow, triggered by the GitLab tag or schedule pipeline. A tag that starts
with `v` creates a stable release; the scheduled nightly pipeline creates a
prerelease on the `nightly-<date>-<sha>` channel.

Install the latest stable release:

```bash
curl -fsSL https://raw.githubusercontent.com/AgentsMesh/YoloShell/main/scripts/install.sh | sh
```

Update to the latest GitHub release:

```bash
yolo update
```

This checks the selected channel, installs a newer immutable image under
`~/.local/lib/yolo/<sha>/yolo`, atomically points `~/.local/bin/yolo` at it,
restarts the local Host and Hub when needed, and probes the live service images,
Hub protocol, storage contract and managed unit pins. Admission must report
`ready`; a failed check exits nonzero. If no services are running it reports the
install and leaves them stopped.

Advanced modes:

```bash
yolo update --check              # check the latest release only
yolo update --admit              # probe the installed image without the network
yolo update --restart            # restart the installed image, then probe
yolo update --no-restart         # install a newer release; activate it later
yolo update --channel nightly    # select the nightly channel
```

`--check` ends with `update available: <version>` or `yolo <version> is already
current`, so callers do not compare the two printed versions themselves.
`--no-restart` leaves the running services untouched and prints
`run yolo restart to activate it` when an installed image is not yet running.

For a non-default profile, pass `--local-profile <file>` to `yolo update`.

Release assets cover macOS arm64, Linux x86_64 and Linux arm64. Linux archives
are built against glibc 2.31 so they run on the supported Linux baseline.
