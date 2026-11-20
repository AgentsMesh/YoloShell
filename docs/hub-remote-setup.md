# Hub-owned remote setup

The entry Hub coordinates broker startup, complete bundle publication, remote
enrollment and local peer configuration. Client connections carry product
management commands; Agent interaction continues through terminal input and
provider hooks. No provider structured API is introduced.

## Startup authority

A registered SSH profile may contain `launch { config_source, source_sha256 }`.
The source is a private local broker configuration; the profile also binds its
resolved configuration revision and broker state directory. Existing registrations
without launch material can continue to use an already admitted broker.

`ConnectSshProfile` creates or reuses the broker under a profile reservation. A
conflicting profile edit cannot cross the active startup transaction. The Hub
retains the source file identity/digest and its own live state owner. A private
socketpair carries a signed, deadline-bounded authorization conversation with the
exact child captured by `ScopedWaitableChild`. Copying the public scope or merely
asserting a PID does not create startup authority.

Commit authorizes one exact handoff; Finished reports that the existing supervisor
actually adopted that same broker. The Hub also waits for the launcher to exit
cleanly and verifies the resulting broker connection. An adopted broker survives
entry Hub exit. The Hub executable embeds the existing broker CLI's three internal
commands, so a remote bundle does not need a fourth executable.

Credential envelopes exist only in bounded memory and private channels. Debug
formatting is redacted. They are absent from the durable setup request, journal,
argv and environment. The desktop JSON bridge represents their bytes as an array,
checks its document limit before dispatch, and leaves unrelated helper limits
unchanged.

## Provisioning and recovery

The local bundle locator contains a complete shared descriptor and root directory.
The reader retains root/manifest/executable identities, validates the current
role inventory, target, size and digest, and supplies immutable byte snapshots.
The broker management protocol carries that locator rather than
binary payloads or an arbitrary command execution interface.

Broker provisioning records the complete request and exact expected remote
selection before its first publication. Its durable phases are Prepared,
Publishing, Published, Enrolling and Complete. Each effect is fenced by the
profile, broker namespace, connection UUID/generation and journal FileIdentity.
Only a fresh owner can take an effect edge. Re-reading the journal cannot recreate
that permission.

Recover only observes the original operation. An unknown publication or enrollment
is never replayed. Explicit Advance can take the never-attempted enrollment edge
from a verified Published state; it cannot retry Publishing or Enrolling. Completed
history is rechecked against the remote enrollment journal and current live state.

The entry Hub's journal separately binds its own identity, the complete setup
request, original local configuration, SSH registration and broker directory
identity. It records Routing before sending the one local Configure mutation.
Local configuration conflicts remain visible and cannot overwrite another edit.
The configured peer retains its original Hub identity and uses explicit SSH
transport; imported Agents do not acquire the entry Hub's identity.

The remote Hub publishes its peer entry point at `<hub-state>/peer.sock`; the
bootstrap receipt returns that absolute path. Remote setup writes the entry
Hub's dial with the selected SSH profile id and that path, and the remote
configuration pins the consumer identity with its export policy. The socket is
derived from Hub state and republished for the service lifetime, so it has no
ephemeral port or binding-record exchange. An occupied or foreign socket fails
startup instead of silently moving the endpoint. Legacy configuration records
that still carry listener evidence are read without it; the retired `listen`
field and TCP dials are dropped from peer entries on migration.

## Client operations

- Start accepts the durable request and starts a bounded Hub-owned task. Closing
  the client does not cancel a task already accepted by the Hub.
- Inspect starts a new read-only check of the retained operation and live remote
  result. It does not start a missing broker or create a journal.
- Poll reads that task's progress/result without starting a new remote check.
  Local profile, journal and configuration evidence are revalidated before a
  retained result is returned. Current Agent reachability remains a separate
  catalogue observation; a setup result does not confer control authority.
  Completed observations have their own bounded retention, separate from active
  task capacity, so starting another operation does not discard an unpolled result.
- Continue requires an existing journal and advances only a known safe step.
  An absent request is rejected. An uncertain Routing step is observed rather
  than submitted again.

Setup tasks use a shared absolute provisioning deadline. Broker provisioning
uses the same budget and watches its RPC connection; cancellation stops entry
into subsequent effect stages and leaves the durable unknown result available.
All setup/profile commands are local to the connected entry Hub, never delegated
through peer resource grants.

## Delivery boundary

The Mac and TUI setup wizard, complete operation-history presentation, Linux native
recovery and release-package gates remain tracked in the project roadmap.
The existing peer editors preserve SSH profile and socket dials, but that is not
a replacement for the complete onboarding interface.

Runtime catalog schema 2 packages the single `yolo` image and its role descriptor
for each Linux architecture; see [bundle catalog](runtime-bundle-catalog.md). An actual Linux artifact build
and package audit are required before those resources can be declared available.
