# Security policy

## Supported versions

Yolo is an Alpha project. Security fixes target the current `main` branch and
the latest published Alpha. Older snapshots may not receive a backport.

## Reporting a vulnerability

Do not open a public issue for a vulnerability. Use GitHub's private
vulnerability reporting for this repository (Security → Report a vulnerability).
If that flow is unavailable, contact the maintainers through a private channel
listed on the repository profile.

Please include:

- affected revision and platform;
- the security boundary that is crossed;
- a minimal reproduction or proof of concept;
- impact and any known mitigations;
- whether the report involves a third-party provider or dependency.

Do not access data, processes or machines that you do not own. Do not run a
denial-of-service test against a shared Hub or another user's terminal.

## Scope

In scope includes PTY custody and process containment, writer leases and agent
authority, plugin trust boundaries, Hub/remote authentication, browser token and
origin policy, creation journals, and secret handling.

Provider binaries (Claude Code, Codex, Gemini, Qwen, Grok), operating-system
sandboxes and third-party dependencies are outside this repository's control;
report those to their vendors as well when relevant.

## Disclosure

Maintainers will acknowledge a report, investigate privately and coordinate a
fix and disclosure date with the reporter. Please leave reasonable time for a
fix before publishing details.

## Hardening notes

- Plugins are trusted user-level processes, not a sandbox.
- LAN web mode uses plain HTTP plus a bearer token and is for trusted networks.
- Browsers must never receive the management socket, raw Hub ID, lease grant,
  creation capability or Hub token.
