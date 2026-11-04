# Yolo upgrade admission

Yolo images are immutable deployments. The CLI symlink is only a selector for
new launches; it is not a migration mechanism for an already-running Host or Hub.

Before a new image opens a Hub state directory, the Hub validates
`runtime-contract.json`. The record binds the current public Hub protocol and
state schema. A missing record is treated as an old or unproven state, and a
mismatched record fails with an upgrade admission error. The new process does not
rewrite identity, catalog, runtime records or creation journals in that case.

The operational upgrade order is:

1. Install the immutable image and verify its digest.
2. Probe the running Host/Hub image, protocol and state contract.
3. Keep the persistent custody domain and live Workers running while the result
   is compatible.
4. Restart only the independently replaceable Host runner and Hub (`yolo restart`
   preserves the custody domain and live Workers).
5. Recheck Hub identity, TerminalAddress, RuntimeEpoch, Worker identity and Agent
   execution identity before accepting the deployment.

Hub startup also performs a service image admission before publishing any
listener. It observes the mapped Hub executable, the live Host through the
Host-build RPC, and the file-helper image, then requires all three SHA-256
images to be identical. A service restart that resolves the mutable CLI link
to a different release therefore fails closed with an actionable mixed-image
error; it cannot expose a partially upgraded Hub. Production unit files should
still use the immutable `/home/<user>/.local/lib/yolo/<sha256>/yolo` path so
each service has an explicit image owner. The link under `.local/bin` remains
for interactive CLI selection only.

If the probe reports incompatibility, `yolo update` exits nonzero with
`upgrade admission refused` plus the failed checks. When the installed image is
newer than the running services, activate it with `yolo restart` or
`yolo update --restart`; `yolo stop` leaves the profile stopped for inspection.
Restarting preserves PTY custody: the Host closes ingress but Workers remain
attached to their PTYs. A restart is refused before stopping anything when
`runtime-contract.json` is incompatible with the installed image, or when the
caller cannot create a new Host (for example an un-delegated Linux shell). There
is no force/apply subcommand: admission never retries an uncertain state mutation
or deletes old state.

`--restart` restarts before probing, so the probe describes the image that was
just activated. Without `--restart`, admission only observes the running
services and never mutates them.

On systemd hosts, each managed unit must pin both the absolute immutable image
path and its exact image metadata:

```ini
ExecStart=/opt/yolo/releases/<digest>/yolo internal hub serve
X-Yolo-Image-SHA256=<sha256>
X-Yolo-Image-Size=<bytes>
```

`yolo update` discovers the local profile, Hub socket and identity, contacts the
running Hub's `GetBuild` endpoint, validates `runtime-contract.json`, and checks
those unit fields. Internal path overrides exist only for deployment tests and
are hidden from the public command help.

The current Linux local TUI applies the same boundary earlier: automatic startup
requires a delegated cgroup-v2 custody root. An un-delegated login shell fails
before raw mode and before durable creation intents. The user can attach through
an existing managed Hub or start the process under a delegated systemd service.
