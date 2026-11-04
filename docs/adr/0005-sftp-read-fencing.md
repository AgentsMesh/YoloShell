# ADR 0005: Fenced SFTP v3 file reads

## Status

Accepted.

## Context

Yolo reads selected remote files for agent discovery and diagnostics. A read is
authorized only after the requested path has been canonicalized into a configured
non-root scope. Reusing that result on a later SFTP channel creates a race: a
remote process can replace a path after REALPATH and before OPEN, causing OPEN to
follow a different symlink or object.

SFTP protocol version 3 provides REALPATH, LSTAT, OPEN, and FSTAT. Its standard
file attributes contain size, UID, GID, permissions including file type, and
second-resolution access and modification times. It does not provide a stable
inode or file identifier, ctime, nanosecond timestamps, or a portable
`O_NOFOLLOW` equivalent for OPEN. OpenSSH extensions do not add a no-follow file
read with a portable stable identity.

Relevant specifications:

- [SFTP protocol version 3](https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-02)
- [OpenSSH protocol extensions](https://github.com/openssh/openssh-portable/blob/master/PROTOCOL)

## Decision

Each authorized read opens a new SFTP channel and executes this protocol:

1. REALPATH the authorized resolved path and require an exact path match.
2. LSTAT that path, require an exact regular-file type, and capture an identity.
3. Require size, UID, GID, full permissions, and mtime to all be present.
4. OPEN the path read-only.
5. FSTAT the handle and require the captured identity.
6. Repeat REALPATH and LSTAT and require the same path and identity.
7. Seek and read no more than the authorized byte count.
8. Repeat FSTAT, REALPATH, and LSTAT and require the same identity and path.
9. CLOSE the handle with an independent deadline.
10. Close the SFTP channel with an independent deadline.

Path drift, symlinks, non-regular files, missing identity fields, or identity
drift fail closed. Bytes remain private to the operation until every post-read
check and the handle CLOSE succeed. Any error after OPEN still attempts bounded
handle and channel cleanup. An earlier operation error takes precedence over a
cleanup error, while a cleanup failure rejects an otherwise successful read.
Access time is deliberately not compared because a read may update it.

The in-memory protocol server binds opaque handles to the object selected by
OPEN. Adversarial tests replace the pathname immediately before OPEN, after OPEN,
and after bytes are read. They also cover symlinks, directories, handle metadata
drift, stalled READ/FSTAT/CLOSE operations, close failure, data suppression, and
cleanup attempts.

## Residual Boundary

This fencing materially narrows but cannot eliminate the race. A malicious
process running as the remote account can replace a file between any two checks
with another regular file having identical size, UID, GID, mode, and
second-resolution mtime, then restore the original before the next path check.
SFTP v3 exposes no stable field that distinguishes those objects. A malicious or
compromised SFTP server can also fabricate internally consistent replies.

Therefore this protocol does not claim the descriptor-rooted guarantee provided
locally by `openat` plus `O_NOFOLLOW` and inode comparison. Eliminating the
remaining same-UID window requires a trusted remote Yolo operation that performs
descriptor-relative resolution and reading inside the deployed runtime, then
returns bytes over an authenticated bounded channel. Until that operation exists,
SFTP file reads are suitable only where the remote account and SFTP server are
inside the trust boundary.

## Consequences

- Servers omitting required SFTP v3 attributes are rejected for file reads.
- Every fence and cleanup operation consumes its own configured deadline.
- Reads incur additional round trips before and after transferring bytes.
- The limitation is explicit and must not be described as race-free path access.
