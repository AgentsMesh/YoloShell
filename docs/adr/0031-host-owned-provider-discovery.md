# ADR 0031: Host-owned provider discovery and AgentStart preflight

## Status

Accepted (2026-11-02). Adds an additive public Hub method and Host capability
facts. No change to the Worker custody, PTY, lease or creation contracts.

## Context

The Web agent picker was a static list of provider ids. A provider that was not
installed on the selected machine was accepted by the Hub: `AgentStart` returned
`awaiting`, created a terminal, and the shell stayed at a prompt with no
provider binding. The user saw a Starting agent that could never bind, and a
direct API caller received the same misleading success.

Provider availability is a property of the launch environment. The Host owns
that environment and already links the compiled launch policies; the Hub and Web
must not infer it from their own `PATH`, a static list, or a filesystem probe on
another machine.

The existing public protocol had no provider descriptors. Adding a field to
`Hello` would break Rust clients that use `deny_unknown_fields`; a new public
method is additive and is naturally ignored by older clients.

## Decision

### Host facts

The Host appends two kinds of capability strings to `HostStatus.capabilities`:

```text
provider_catalog                  provider facts are present
provider.<provider-id>            this launch executable is available
```

`provider-id` is the same id AgentStart accepts (`codex`, `claude_code`,
`qwen_code`, `gemini_cli`, `grok`). Availability is computed from the
compiled launch policies and the Host's effective `PATH`; it means the
executable is present and executable, not that credentials or provider login
are valid. The Host never exposes the executable path or environment to the Hub.

`provider_catalog` is always present on a Host that supports this contract,
including when no provider is installed. This distinguishes "no provider facts"
from "all providers unavailable".

### Public `list_providers`

The Hub adds `ListProviders {}` with `Observe`/`ReadOnly` access and a bounded
reply:

```text
Reply::Providers {
  providers: [
    { provider_id, display_name, available }
  ]
}
```

The list is sorted by `provider_id`, validated and capped at 64. It contains no
Hub id, executable path, environment, credential or launch argv. A peer-addressed
call follows the ordinary public command forwarding path; a peer Hub answers
with its own Host facts.

`ProviderDispatcher` sits inside `ClientAccessRuntime`, so the normal client
authorization check runs before provider handling. It intercepts only local
`list_providers` and local `agent_start`; every other command is delegated
unchanged.

The Hub CLI supplies the compiled provider descriptors from its provider
runtime (kind as provider id, catalog display name). The dispatcher combines them
with the HostStatus facts:

- no `provider_catalog`: `list_providers` returns `Unsupported`, so old Hosts
  and old peers fall back to the client's legacy static list;
- catalog present: every descriptor is returned with `available` true/false.

### AgentStart preflight

For a local `agent_start`, when the Host advertises `provider_catalog` and the
requested provider has no available fact, the dispatcher returns
`Unsupported` before delegating. No terminal is created. When provider facts are
absent, the command is delegated as before for compatibility.

### Web projection

`MachineObserver` calls `list_providers` only when the machine's Hello advertises
the method, on the same 60-second refresh as Hello. The validated list is part
of `MachineView` and therefore part of catalogue snapshots and per-machine
patches. The New agent dialog renders the selected machine's advertised list:

- available providers are selectable, unavailable entries are disabled and
  labeled `· unavailable`;
- a machine with no available provider cannot start an agent;
- an older Hub with no advertised list keeps the static fallback.

The fallback list was corrected to the real AgentStart ids and catalog names:
`gemini_cli` (Gemini CLI) replaces the invalid `gemini`, and Grok uses its
catalog label. The creation endpoint accepts the corrected ids.

## Consequences

- An unavailable provider fails before terminal creation, for both browser and
  API callers of a new Hub.
- Availability becomes visible per machine without a Hub-side PATH guess or a
  browser filesystem probe.
- Installing a provider is observed on the next Host refresh; a login/credential
  failure can still occur after a positive availability result.
- Old Hubs remain usable: the method is additive, and lack of facts keeps the
  existing behavior.
- The Hub's public method and Host capability strings are additive; no existing
  reply shape changed.
- The provider list is per machine, so a federated picker no longer offers a
  provider merely because another machine has it.

## Evidence

Rust tests cover launch-policy entries, Host executable probing, provider reply
validation/bounds, dispatcher projection/preflight, and unsupported facts. Web
tests cover advertised/unavailable/malformed provider lists and fallback.
Live acceptance preserved existing PTY runtime epochs, returned five providers,
and verified unavailable-provider rejection without terminal creation. Live
acceptance evidence is retained with the source revision.
