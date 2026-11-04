# Acceptance Receipt Operator Contract

> Historical release-tooling contract. The current product has no macOS app.

This document defines the production acceptance receipt consumed by
`verify-acceptance` and `distribute`. The receipt is an external attestation for
one sealed candidate. It is not generated or signed by repository code.

## Trust Boundary

The signing key is the private Ed25519 key corresponding to the public key
pinned in `acceptance_receipt_constants.rs`. Keep that private key only in the
offline authority or HSM. Never place it in this repository, a candidate,
`.yolo-dev`, an environment variable, a command-line argument, a log, or a test
fixture. Before signing, the authority must derive the raw 32-byte public key
and verify its SHA-256 is:

```text
f3ced7ab2eeb81f164a2d61464902ddfdeb958b60ee9ec6c3b950ca4b7e541dc
```

The operator completes every item in [Alpha verification approach](alpha.md#verification-approach)
against the exact candidate. The authority signs only the resulting record and
does not copy any private material back to the release workstation.

## Envelope Schema

The JSON member order shown below is mandatory. This example is structurally
complete, but its zero signature is deliberately invalid and must never be used
as a receipt. Replace the candidate binding, observed macOS versions, operator,
times, and signature with evidence from the actual acceptance run.

```json
{
  "payload": {
    "schema": 1,
    "authority": "yolo-release-acceptance-v1",
    "algorithm": "ed25519",
    "record": {
      "schema": 1,
      "product": "Yolo",
      "source_revision": "0000000000000000000000000000000000000000",
      "artifact_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "provenance_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "checklist_schema": "yolo-visible-ui-v2",
      "providers": [
        {"provider": "claude_code", "version": "2.1.241 (Claude Code)"},
        {"provider": "codex", "version": "codex-cli 0.150.1"}
      ],
      "desktop_os": {"name": "macOS", "version": "14.2", "architecture": "arm64"},
      "remote_os": {"name": "macOS", "version": "14.2", "architecture": "arm64"},
      "operator_identity": "operator@example.test",
      "completed_at_unix_seconds": 1800000000,
      "expires_at_unix_seconds": 1800003600,
      "results": [
        {"identifier": "normal_visible_launch", "status": "passed"},
        {"identifier": "title_bar_dragging", "status": "passed"},
        {"identifier": "split_controls", "status": "passed"},
        {"identifier": "terminal_input_and_rendering", "status": "passed"},
        {"identifier": "real_codex_detection_status_and_recovery", "status": "passed"},
        {"identifier": "real_claude_detection_status_and_recovery", "status": "passed"},
        {"identifier": "remote_macos_multiplexed_recovery", "status": "passed"}
      ]
    }
  },
  "signature": "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
}
```

All fields are required; duplicate and unknown fields are forbidden. Git SHA-1
and SHA-256 object IDs are accepted for `source_revision`; digests are exactly
64 lowercase hexadecimal characters. `artifact_sha256` is
`artifact.sha256` from `Yolo.provenance.json`. `provenance_sha256` is SHA-256
of the producer's canonical pretty JSON provenance bytes, including its final
LF. For an unmodified verified candidate this is the digest of the exact
`Yolo.provenance.json` file.

Provider entries and results must have exactly the shown values and order.
Both platforms must be `macOS` on `arm64`, with a numeric version at least
14.2. `completed_at_unix_seconds` is the UTC completion time, cannot be in the
future when verified, and must precede expiry. Expiry must be no more than 31
days after completion and verification must occur strictly before expiry.

## Canonical Signing

1. Construct the typed `payload` object in the exact member order above.
2. Serialize only that object, beginning with `{"schema":1,...}`, as compact
   UTF-8 JSON with no insignificant whitespace and no trailing LF.
3. Sign those exact payload bytes with Ed25519. Encode the 64-byte signature as
   exactly 128 lowercase hexadecimal characters.
4. Serialize the envelope as compact UTF-8 JSON in `payload`, `signature`
   order, then append exactly one LF byte. Do not pretty-print the signed file.
5. Transfer only the completed envelope to the release operator. Any later
   reformatting, newline conversion, member reordering, or field change makes
   the signature or canonical encoding invalid.

The authority should retain the candidate binding, checklist evidence,
operator identity, completion and expiry times, public-key fingerprint, and
final receipt SHA-256 in its external audit record.

## Release Workstation Metadata

Install the received bytes into a new file created by the same effective user
that will run Bazel. Copy the bytes; do not hard-link the authority's file. On
macOS, remove inherited extended ACL entries and set the final mode only after
the receipt is complete:

```bash
receipt_path=.yolo-dev/acceptance.json
/bin/chmod -N "$receipt_path"
/bin/chmod 0400 "$receipt_path"
test ! -L "$receipt_path" && test -f "$receipt_path"
test "$(/usr/bin/stat -f '%u' "$receipt_path")" = "$(/usr/bin/id -u)"
test "$(/usr/bin/stat -f '%Lp' "$receipt_path")" = 400
test "$(/usr/bin/stat -f '%l' "$receipt_path")" = 1
```

The verifier opens the final component with `O_NOFOLLOW` and requires a regular
file owned by its effective user, permission mode exactly `0400`, exactly one
hard link, no extended ACL, and size at most 64 KiB. `chmod -N` is required when
the source or parent may supply an inherited ACL; mode bits alone do not prove
ACL absence. Recreate a file with the wrong owner or link count under the
release user rather than using `sudo`, `chown`, or a hard link. The verifier is
the authoritative metadata and content check.

Run the read-only check before granting access to signing or notarization:

```bash
bazel --batch run --jobs=1 //plugins/macos:YoloReleaseEvidence -- verify-acceptance \
  --candidate-directory .yolo-dev/release-candidate \
  --acceptance-receipt "$receipt_path"
```

Only the same unchanged path should then be passed to `distribute`. The tool
retains its descriptor and rechecks identity, metadata, bytes, candidate
binding, and signature across the distribution transaction.
