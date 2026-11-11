# PTY child environment

The execution machine's Worker owns the environment used to start a PTY child.
Creation and agent resume both reach `bootstrap_command`, which applies the
same `worker::environment` policy before the bootstrap gate starts the shell or
agent. Attaching a client does not rebuild an existing process environment.
TUI input remains UTF-8 bytes; it does not compensate for shell locale settings.

## Resolution

1. `CommandBuilder` snapshots inherited OS environment values on the execution
   machine. Keep them as OS strings, including values that are not Unicode.
2. Apply the launch's explicit environment entries to that snapshot. An empty
   entry replaces an inherited value; it does not reveal the inherited value.
3. Fill missing or empty `TERM`, `COLORTERM` and `TERM_PROGRAM` with Yolo's
   terminal capability defaults. Resolve these against the merged snapshot.
4. If `LC_ALL`, `LC_CTYPE` and `LANG` are all missing or empty, set only
   `LC_CTYPE`: `C.UTF-8` on Linux, `en_US.UTF-8` on macOS.

The standard locale precedence remains `LC_ALL > LC_CTYPE > LANG`. A launch
override has priority for the same key; a launch `LANG` does not override an
inherited `LC_ALL`. Explicit `C`, `POSIX`, non-UTF-8 and invalid locale values
remain the user's selection. Empty locale values are treated as unset, as in
POSIX locale resolution. This policy does not validate installed user-selected
locales or replace explicit Readline settings.

Only the character category receives a default. No `LC_ALL` override, process
global `setlocale`, shell startup file editing, or environment transfer from a
client machine is involved. Numeric/date settings, credentials and agent
configuration keep their existing ownership. Defaults are selected on the
machine running the PTY, including when the client connects through another Hub.

## Failure this prevents

A system service or GUI launcher can have no locale even when the user's login
shell has one. In the C locale Bash Readline's `convert-meta` can interpret UTF-8
bytes as Meta shortcuts. For example, the bytes of `费` are `E8 B4 B9`;
interpreting the high bit as Meta yields `Esc h`, `Esc 4`, `Esc 9`, with the last
two selecting numeric argument 49. The result is `(arg: 49)` instead of text.
Setting a UTF-8 character locale before Bash initializes prevents this without
rewriting keyboard input or hiding the error in the renderer.

Tests cover sparse service environments, empty/explicit values, same-key and
cross-key precedence, non-Unicode values and bootstrap wiring. Real private
Bash PTYs exercise Chinese input and Linux bracketed paste, with explicit C
locale as a failing-input control. TUI encoder tests separately retain exact
UTF-8 bytes and enforce byte-based paste bounds.

Existing shells retain their current environment after an update. A new PTY
uses the new policy. For an existing affected Bash, the user can cancel a pending
Readline numeric argument with Ctrl+G, then run:

```sh
export LC_CTYPE=C.UTF-8
bind 'set convert-meta off'
```

This recovery assumes `LC_ALL` is unset, as it was in the reported Ubuntu
service. Yolo does not inject recovery commands into an active user's PTY.
