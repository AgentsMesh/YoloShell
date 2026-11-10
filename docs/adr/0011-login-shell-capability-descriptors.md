# ADR 0011: Capability Descriptors Across Login Shells

## Constraint

The system Bash shipped by Apple enables `CLOSE_FDS_AT_LOGIN`. Its startup code
closes descriptors from 3 through 19 for an interactive login shell, even when
those descriptors do not have `FD_CLOEXEC` set. Clearing close-on-exec alone is
therefore not sufficient to pass the process capability into the user's shell.

Source: [Apple Bash shell.c](https://github.com/apple-oss-distributions/bash/blob/main/bash-3.2/shell.c),
the `CLOSE_FDS_AT_LOGIN` block.

## Decision

On macOS, a transferred capability writer uses a descriptor at or above 20.
`F_DUPFD_CLOEXEC` selects an unused descriptor without overwriting an existing
resource. The original descriptor is closed through its owned handle. The pipe
object and its reader/writer identity are unchanged.

The writer remains close-on-exec until the existing guarded exec operation.
Failure recovery also retains a backup outside the login-shell close range.
Allocation errors fail before capability acknowledgement; there is no fallback
to the shell-reserved descriptor range.

## Verification

The integration probe passes a real capability through direct exec and through
interactive login `/bin/sh` and `/bin/zsh`. The final helper verifies possession
of the original kernel pipe writer identity, without requiring its original FD
number. Existing exec-failure tests continue to check inheritance serialization
and close-on-exec restoration. A genuine premature writer release remains an
ownership fault; no timer or negative descriptor scan grants absence authority.
