# Release Candidate Evidence (retired macOS app path)

Resource lifetime and cleanup guarantees are specified in [Release resource custody](release-resource-custody.md).
`//plugins/macos:YoloReleaseEvidence` is the only supported candidate-evidence entry
point. Its producing operation accepts a clean Git workspace and one candidate
output path. The Bazel wrapper supplies the workspace; callers supply only the
output path. Callers cannot inject an archive, audit script, policy, gate
command, receipt, version assertion, or completion assertion.

Create a private parent directory. The final candidate directory must not exist:

```bash
/bin/mkdir -m 700 .yolo-dev
bazel --batch run --jobs=1 //plugins/macos:YoloReleaseEvidence -- candidate \
  --output-directory .yolo-dev/release-candidate
```

Use a new candidate name when `.yolo-dev/release-candidate` already exists.
The command deliberately refuses to replace a previous candidate, including a
directory or symlink created concurrently.

One process performs this fixed sequence:

1. Resolve one clean Git root and exact revision with replacement objects
   disabled.
2. Materialize the revision into a private bare Git database as a complete
   private pack, validate it strictly, and export only from that database. The
   private database has no alternates, live refs, live repository-config input,
   replacement refs, or live `.git/info/attributes` dependency. Normal
   repositories and linked worktrees with a `.git` gitfile use the same flow.
3. Reject symlinks and special files, remove every source write bit, retain
   directory descriptors, and seal the exported source manifest.
4. Copy the pinned macOS arm64 Bazel 8.2.1 executable through retained
   descriptors, reopen it read-only, verify its SHA-256, seal its private
   directory as owner-immutable, and run a bounded version check. macOS does
   not expose `fexecve` or `execveat`, so launches use that sealed private
   pathname and verify its descriptor identity and directory seal before and
   after execution.
5. Resolve digest-pinned Linux Bazel 8.2.1 binaries and the Bazel-fetched,
   digest-pinned macOS arm64 Docker client. Revalidate their path, descriptor
   identity, digest, and version around every use. Only the Docker engine is an
   ambient prerequisite.
6. In digest-pinned amd64 and arm64 images, build optimized `//core:yolo`,
   verify its version/ELF, and export the executable. A separate delegated-cgroup
   container runs complete `//core:tests` and kernel custody tests as a nonroot user.
7. Seal both ELFs and their manifest as a private Bazel repository. Build
   optimized `//plugins/macos:YoloRelease` with that exact repository override, bind
   `bazel-bin` to the retained output root, and pin `plugins/macos/YoloRelease.zip`.
8. Audit the pinned archive and run release unit, packaged E2E, Rust coverage,
   and Swift coverage gates against the same snapshot and artifact repository.
   The E2E launch is inactive and never requests focus. Coverage gates consume
   only reports copied into sealed custody. The Swift gate also emits a
   deterministic source-branch audit from SwiftParser syntax and exact coverage
   region-entry counters; an active source arm without evidence fails the gate.
   Its coverage boundary is shipped `plugins/macos/Sources` app code; `plugins/macos/Tools` is outside that metric.
9. Stage the archive, checksum, and schema-9 provenance as one complete read-only candidate, then publish them with one exclusive atomic install.

Each inner Bazel invocation uses `--batch`, the fixed serial `--jobs=1` limit, a private output root and symlink prefix, isolated `HOME` and `TMPDIR`, a fixed environment, and only the tracked `.bazelrc`. System, home, and implicit workspace rc files are disabled.
`--batch` describes those invocations; it is not a claim that no unrelated Bazel process or daemon exists elsewhere on the host.

Every gate binds to the exact source revision and tool identity it executes. Release bundle audit and packaged recovery E2E consume `YoloRelease.zip` built with the sealed Linux repository; artifact audit separately binds the pinned archive digest to its retained manifest, policies, auditor, and both ELF digests.

The installed directory contains exactly:

```text
Yolo.zip
Yolo.zip.sha256
Yolo.provenance.json
```

The parent directory must exist, be owned by the current user, must not be group or world writable, and must have no extended ACL. Its pathname identity and security policy are revalidated before staging, before install, and before success. The candidate output directory must initially be absent.

Publication retains the parent and three output descriptors, requires one hard link and no extended ACL on every mode-`0400` file, and rejects extra entries. It claims a nonblocking exclusive lock opened from the retained parent; competing producers fail closed instead of waiting without a deadline. The mode-`0700`, flag-free staged directory is installed once without replacement, synced, reopened, and fully verified. Only then is it changed to mode `0500` with `UF_IMMUTABLE`. The candidate and parent must both complete macOS `F_FULLFSYNC`; unsupported or failed barriers fail closed without an ordinary-`fsync` fallback.
Before them, the process captures the exact marker-bearing candidate seal, parent
seal, formal binding, and opened file identities. Both barriers make the
transaction immediately irrevocable. `CommitMarked` is emitted only after those
exact pre-barrier bindings remain unchanged.

Before the durable state, ordinary errors roll the formal name back to staging.
An interruption before `fchflags` can leave a formally named directory that
standalone verification rejects for lacking the required mode and flag. Candidate
or parent barrier errors are also precommit: rollback clears the flag, restores
staging, and requires a full parent barrier. A rollback barrier failure is
reported with the original error instead of being treated as committed.

After `fchflags` makes the flag visible but before `F_FULLFSYNC` returns, an
interrupted operation has no `CommitMarked` acknowledgement. Standalone integrity
verification can report current state but cannot authenticate a historical
full-sync call, so consumers must require a successful producing operation.

The crash matrix makes this explicit: `MarkerApplied` precedes the candidate
barrier and `CandidateBarrierComplete` precedes the parent barrier. Their live
paths can verify but have no durable acknowledgement. `DurabilityBarriersComplete`
runs after both barriers and the irrevocable transition. Any later seal, binding,
namespace, policy, or content discrepancy is preserved as validity uncertainty.

After `CommitMarked`, an acknowledgement failure is reported as such only when
repeated final validation still succeeds; that candidate remains independently
verifiable. A path, namespace, policy, or content failure during final
revalidation is instead reported as committed-but-invalid-or-uncertain and does
not claim that standalone verification succeeds. Every valid committed output
still contains exactly the three files listed above.

Candidate archives are limited to 512 MiB, general release digest inputs to
1 GiB, and evidence files to 1 MiB. Hashing and copying capture one initial
regular-file length, read exactly that length, and reject truncation, growth,
identity changes, or timestamp changes instead of following a moving EOF.

## Trust Boundary

The candidate pipeline executes code from the selected repository revision as
part of its build and tests. Run it only on a workspace and revision whose
contents are trusted for local code execution. Namespace isolation, fixed gate
selection, bounded process supervision, and post-command manifest checks defend
the evidence pipeline from ambient state changes; they do not sandbox arbitrary
repository code.

The outer Bazel wrapper and producer executable are built from the live workspace
before the producer creates its private revision snapshot. They are part of the
trusted bootstrap. A caller cannot use that producer to prove that the producer
itself was not maliciously replaced, and the evidence is not a self-attestation
of the live workspace. Index entries marked `skip-worktree` or
`assume-unchanged` are rejected, including in linked worktrees, but this check
does not turn an untrusted same-user workspace or producer into a trust root.

Process supervision starts every command with `POSIX_SPAWN_START_SUSPENDED` and captures its immutable macOS kernel identity. After the handoff commits, management custody retains only the discovery reader; the launcher, exact root,
and descendants inherit only the writer. The pre-exec acknowledgement binds the
exact root identity and both nonzero, distinct reciprocal endpoint handles,
proves that exact root holds the writer, then drops the issuer's writer while the
spawn gate is still held.

Descriptor-backed discovery scans the complete stable same-UID inventory,
including unrelated or pre-root processes that can receive the writer through
`dup` or `SCM_RIGHTS`; such a transfer extends custody instead of escaping it. Every member proves the exact writer handle; environment evidence only discovers or classifies candidates and cannot prove membership, exclusion, or settlement. A matched or excluded parent cannot authorize or exclude its child. Writer presence keeps
discovery publication pending. Explicitly closing the writer before registration relinquishes membership. Premature EOF and indeterminate release observations are sticky poison and later writer closure cannot turn either into
clean release. Clean pipe EOF after exact root exit closes the process-discovery membership universe because queued `SCM_RIGHTS` copies and inherited descendants retain writer custody. After EOF, Yolo queries no ambient environments and only replays exact identities retained from writer-present scans until their exact generations exit. It is not proof that every PTY descriptor is gone.
Released PTY scans therefore use the complete same-UID inventory only to discover exact positive holders. An environment-scrubbed candidate is still scanned, and a matched live generation remains owned until it exits.
Sequential negative scans remain `Pending` regardless of candidate order, repeated inventories, fingerprints, or sweep count: PTY custody can move between candidates or remain queued in an `SCM_RIGHTS` message without appearing in any process descriptor table.
Only a future kernel-global PTY exclusion primitive may authorize empty completion. Post-inventory event and watcher-audit fences still restart positive reconciliation.
Termination combines process-group signaling with exact signaling of discovered identities.
Settlement additionally requires typed containment or custody absence plus stdout and stderr EOF. A released-PTY negative scan cannot supply that absence authority; cleanup remains visibly blocked.
Cleanup errors accumulate with the original failure while later cleanup and reaping run.

One accepted containment limit is explicit. Synchronous regular-file hashing and
copying fence file type, identity, captured length, byte count, timestamps, and
maximum size, but do not provide a wall-clock deadline against a malicious or
indefinitely blocking filesystem. Run the pipeline only when the workspace,
Bazel executable and cache, temporary storage, and candidate parent are on
trusted, responsive filesystems. The blocking-filesystem limit is an accepted
containment residual; discovery uncertainty fails closed instead of
publishing an unproven empty result.

The output records source-bound gates and a descriptor-bound artifact audit in
one candidate operation. It includes packaged-artifact recovery E2E but does
not establish reproducible builds; a second build can still differ because not
every permitted toolchain input is represented as a reproducibility proof.

## Integrity Verification

The non-producing operation opens the parent and candidate with retained
`openat` descriptors, checks the sealed directory and exact entry set, and
checks all three fixed files through their initially opened descriptors:

```bash
bazel --batch run --jobs=1 //plugins/macos:YoloReleaseEvidence -- verify \
  --output-directory .yolo-dev/release-candidate
```

Verification checks the artifact digest and size, checksum, schema, distinct
artifact-audit binding, source-bound gate shape, fixed command evidence, shared
execution context, and pending manual acceptance. It records
`scope: artifact_integrity_only` and
`gate_execution_authenticated: false`.

Integrity verification cannot authenticate historical gate execution. Anyone able
to replace all three coherent files can forge the unsigned candidate evidence.
Historical authentication uses a canonical JSON acceptance receipt signed with
Ed25519 by the repository-pinned authority. The receipt binds the source
revision, artifact and provenance digests, checklist schema, exact provider
versions, desktop and remote platforms, operator, completion time, and bounded
expiry. Verification rejects missing, malformed, non-canonical, untrusted,
expired, or differently bound receipts; without a valid receipt the candidate
remains `candidate_unaccepted` and public distribution fails closed.

```bash
bazel --batch run --jobs=1 //plugins/macos:YoloReleaseEvidence -- verify-acceptance \
  --candidate-directory .yolo-dev/release-candidate \
  --acceptance-receipt .yolo-dev/acceptance.json
```

This check is read-only: it never stages, extracts, signs, notarizes, or publishes.
Independent package publication used the retired macOS app delivery path.

Generated provenance uses schema version 9 and explicitly records
`release_status: candidate_unaccepted`. Every `yolo-visible-ui-v2` check remains
pending. Visible launch, title-bar dragging, split controls, terminal input,
real Codex and Claude recovery, and remote macOS recovery are release-blocking.
Candidate evidence is not signing or notarization; that path used the retired
macOS app delivery workflow.
