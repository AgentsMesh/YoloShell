# ADR 0022: Third-party plugin distribution and approval

Status: Superseded by ADR 0023 (trusted-only plugins); historical (2026-10-31). Supersedes ADR 0020's deferral of third-party
code and ADR 0021's compiled-in-only scope for third-party plugins.

## Context

ADR 0020 and ADR 0021 established a Host-supervised plugin process with
broker-mediated grants. They deliberately deferred third-party native/WASM
code, signature verification, remote distribution and the public ABI. The
product now requires third-party native and WASI plugins to load and run
through the same install, approve and invoke path without exposing Yolo's
internal Cap'n Proto protocol.

The security boundary remains the custody model: the Host is the only
executor, the Hub authorizes every invocation, the plugin has no Hub/Host/
Worker socket or client key, and every effect leaves the process through a
grant-authorized broker call.

## Decisions

1. **Public protocol, not Cap'n Proto.** Third-party plugins exchange one
   4-byte big-endian length-prefixed UTF-8 JSON document over stdin/stdout per
   process. `protocol: 1` is independent of internal wire versions. Cap'n
   Proto remains Yolo-internal and is never documented or handed to a
   third-party plugin.
2. **Canonical bundle.** `.yplugin` is one JSON document containing the
   manifest, path/mode/SHA-256/base64 file entries and an optional Ed25519
   publisher signature. The bundle digest is canonical over format, manifest
   and file entries; local directory installs and downloaded files normalize
   to the same digest. Path traversal, symlinks, unknown fields, oversized
   documents and non-`0644`/`0755` modes are rejected.
3. **Manifest v4 runtimes.** A server manifest declares `compiled_in`,
   `native` or `wasm`. Compiled-in keeps `YoloImage` and the internal
   protocol; external runtimes require a bundle-relative entrypoint and public
   protocol 1. WASI entries are platform-independent `wasm32-wasip1` command
   modules.
4. **Approval binds the external artifact, not the Yolo image.** A compiled-in
   approval continues to bind the image digest. An external approval binds
   `manifest_digest + bundle_digest + plugin_protocol + grant`; a Yolo image
   update does not require external reapproval.
5. **Low-friction updates.** An update signed by the pinned publisher
   auto-approves only when the requested grant, actions, effects, contexts,
   argument schemas, contributions, limits and runtime target do not expand
   beyond the approved revision. Unsigned updates, a different publisher,
   expanded contracts or incompatible protocols move to
   `ReapprovalRequired`. `plugin update --approve` installs and approves an
   unsigned update in one command using the existing grant or an explicit
   grant document.
6. **Mandatory isolation, no fallback.** Linux external plugins require
   bubblewrap plus seccomp and, when the canonical Host configures it, a pids
   cgroup. macOS external plugins force a seatbelt profile that denies
   network, fork and writes outside the data directory, and scopes execution
   to the bundle plus system runtime paths. macOS reads remain broad because
   firmlink and dyld shared-cache resolution make path allowlists brittle.
   Missing isolation fails closed as plugin unavailable. Rlimits bound CPU,
   address space, file size, descriptors and core dumps. Invocation timeout,
   disable/revoke, client disconnect and Host drop terminate the verified
   process group; the Hub also sends `plugin_cancel` by invocation request id.
7. **Remote source is one HTTPS URL.** The CLI downloads through an optional
   absolute `curl` executor with argv-only invocation, a scrubbed environment,
   HTTPS-only redirects, bounded size and optional CA bundle. The Hub stores
   the URL and bundle digest for `plugin update`.

## Consequences

- First approval of an external plugin is an explicit operator action with a
  concrete grant. Subsequent compatible signed updates can stay active; every
  expansion still requires the operator.
- Restarting the Host runner changes no external approval. Compiled-in
  approvals still follow the image digest.
- The Host verifies containment, regular-file identity and the entry digest
  immediately before launching. Verification and execution use separate path
  resolution, so a same-UID writer could still race the entry between the two
  operations; fd-exec is the durable fix and remains future work.
- Native plugins run as the same UID inside namespaces/seatbelt. They can
  still execute other programs available inside the sandbox; the Linux
  bubblewrap filter cannot deny the initial exec, and the macOS seatbelt
  profile is deprecated. WASI modules are sandboxed by wasmi and the runner's
  preopened directories.
- `RLIMIT_NPROC` is not lowered per plugin: it is a per-user limit, and a low
  value also blocks unrelated same-user processes. Process-count containment
  relies on the pids cgroup, wall timeout and process-group kill.
- No marketplace, registry, publisher revocation list or key rotation flow is
  introduced. A new publisher key, unsigned bundle or expanded contract is a
  reapproval event.
