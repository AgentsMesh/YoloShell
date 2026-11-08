# SSH runtime bundle deployment

`RemoteBundleInstaller` composes the typed bundle staging API with the native
release-store commit CLI. The caller supplies an authenticated SSH transport
whose endpoint and host key have already been admitted by its connection policy.
Use a canonical remote installation root with existing verified ancestors.

Construct `BundleStageRequest` from the complete local descriptor and the runtime
payload. Its constructor verifies size, digest, and binary target before any
remote command can execute. `prepare` reads the expected remote selector without
uploading; persist that request together with its installation root and local
bundle reference when recovery must survive a caller crash.

`publish` checks the remote target, rechecks the expected selector, stages the
complete bundle, and probes the executable's exact product name/version. It
then invokes the verified staged `yolo internal hub` exactly once with `--install-bundle`, a
separate `--install-root`, and bounded internal `--request-capnp` (base64 without padding). The existing
command policy passes these values as shell-quoted arguments; no generic command
stdin transport is added.

The native Hub owns the durable directory/selector transaction. SSH upload does
not claim that SFTP can sync directory entries or atomically publish a release.

When a commit response is lost or malformed, recovery verifies the inspection
runtime against known local bytes using bounded SFTP reads. It tries the immutable
release path first, then the retained stage path. Only a verified executable may
run `internal hub --inspect-install`. An inspection confirms success only when the exact
descriptor, operation, selector revision, and derived release path match the
original request. A newer selection or unconfirmed result never triggers another
commit. `UncertainPublication` retains the original request for later inspection.

`recover` is explicitly read-only and may be called after reconnecting. Callers
must not replace uncertain publication with a new `install` request. Completed
stages are retained when later probes or commits fail. Failed or cancelled SFTP
uploads retain unproven stage entries with bounded diagnostic records: SFTP v3
cannot prove inode ownership, so matching bytes do not authorize deletion.
Cancellation closes the owned channel while preserving sibling Hub/PTY channels.
No old release is deleted by this component.
