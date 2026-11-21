# ADR 0017: Observation authority tiers

## Status

Proposed on 2026-10-30. Extends ADR 0015; not implemented.

## Context

ADR 0015 gives every supported provider one authority: authenticated provider
hooks and bindings. Providers without hooks are visible only as process
observations, and the projection reports `Unknown` activity. A declared tier
model can cover full lifecycle hooks, session-identity hooks and screen-only
detection, arbitrating between them in one place. The tier model is worth
adopting; screen-derived authority is not.

## Decision

Make the observation tier an explicit, declared property of each provider
rather than something inferred from "has an adapter":

| Tier | Source | May set |
|---|---|---|
| `HookFull` | binding + turn journal | lifecycle, activity, capabilities, control witness |
| `HookIdentity` | session identity only | identity and recovery; activity falls to a lower tier |
| `Process` | worker/process identity | reachability and liveness only |
| `Terminal` | Worker-derived activity summary | activity hint only |

Rules:

1. **Precedence and release.** A live hook authority wins over any lower tier.
   Process exit clears the matching hook authority; a newer execution
   supersedes an older one; a report older than the authority it belongs to is
   suppressed and recorded as a diagnostic, never published.
2. **Fallback never grants control.** A `Terminal` observation can never
   produce a control witness, an interrupt capability, or a receipt; the
   existing capability gate already requires a witness.
3. **No screen bytes leave the Worker.** The `Terminal` tier carries a derived
   summary only: foreground agent, live prompt, permission dialog, working
   indicator, pager/transcript. Raw text, titles from screen content, and
   screen output reads stay off the control plane.
4. **Viewer safety.** A terminal showing history or a pager must not be read
   as idle; the summary reports "not live" and the tier yields unknown
   activity.
5. **Display is not state.** Titles, labels and metadata never feed lifecycle,
   reachability or control.

This requires amending ADR 0015's "observation reads no screens" to
"observation reads no screen bytes; a derived activity summary is a labelled,
non-authoritative fact".

## Deliberately not adopted

- Screen text as state or control authority, including regex output waits.
- A single coarse status enum that conflates lifecycle, activity, approval and
  turn end (`Done` is an event, not a state).
- Unauthenticated hook reports and free-form source labels; our hook input
  keeps its binding/authority digests and runtime identity.
- Screen-derived titles and `agent.read` exposing raw terminal text to other
  agents (prompt-injection and secret-exposure surface).

## Consequences

- The ProviderRuntime row gains its tier; the manifest and projection policy
  derive from it instead of a boolean.
- The Worker gains a derived-summary observation method used only when no
  higher authority is live; it never returns text.
- New providers can ship at the `Terminal` tier first and earn higher tiers as
  hooks are implemented, without weakening the typed control plane.
