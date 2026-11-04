pub const GIT_QUIET_ARGUMENT: &str = "--quiet";
pub const GIT_HARD_RESET_ARGUMENT: &str = "--hard";
pub const GIT_LOOSE_OBJECT_PREFIX_LENGTH: usize = 2;
pub const LINKED_WORKTREE_NAME: &str = "linked-worktree";
pub const TAR_EXTRACT_STDOUT_ARGUMENTS: &[&str] = &["-xOf"];
pub const BAZEL_OUTPUT_REVISION_A: &str = "revision-a";
pub const BAZEL_OUTPUT_REVISION_B: &str = "revision-b";
pub const RUST_COVERAGE_BYTES: &[u8] = b"TN:yolo\nSF:core/src/lib.rs\nend_of_record\n";
pub const OTHER_RUST_COVERAGE_BYTES: &[u8] = b"TN:other\nSF:core/src/main.rs\nend_of_record\n";
pub const PINNED_BAZEL_TEST_PATH: &str = "/private/tmp/pinned-bazel";
pub const TRUSTED_BAZEL_CANONICAL_TEST_PATH: &str = "/private/trusted/bazel";
pub const TRUSTED_BAZEL_EXECUTION_TEST_PATH: &str = "/private/pinned/bazel";
pub const LINUX_BAZEL_EXECUTION_TEST_PATH_PREFIX: &str = "/private/pinned/bazel-";
pub const ARTIFACT_SHA256_JSON_KEY: &str = "artifact_sha256";
pub const SOURCE_REVISION_JSON_KEY: &str = "source_revision";
pub const RUNTIME_MANIFEST_SCHEMA_JSON_KEY: &str = "schema_version";
pub const RUNTIME_MANIFEST_RUNTIMES_JSON_KEY: &str = "runtimes";
pub const RUNTIME_MANIFEST_ARCHITECTURE_JSON_KEY: &str = "architecture";
pub const RUNTIME_MANIFEST_PLATFORM_JSON_KEY: &str = "platform";
pub const RUNTIME_MANIFEST_IMAGE_JSON_KEY: &str = "image";
pub const RUNTIME_MANIFEST_BAZEL_DIGEST_JSON_KEY: &str = "bazel_sha256";
pub const RUNTIME_MANIFEST_VERSION_JSON_KEY: &str = "version";
pub const RUNTIME_MANIFEST_BUNDLE_PATH_JSON_KEY: &str = "bundle_root";
pub const RUNTIME_MANIFEST_DESCRIPTOR_JSON_KEY: &str = "descriptor";
pub const BUNDLE_TARGET_JSON_KEY: &str = "target";
pub const RETIRED_BUNDLE_ARTIFACT_ROLES: &[&str] = &["host", "hub", "file_helper"];
pub const RETIRED_BUNDLE_SCHEMA_VERSION: u16 = 1;
pub const BUNDLE_BUILD_ID_JSON_KEY: &str = "build_id";
pub const BUNDLE_ARTIFACTS_JSON_KEY: &str = "artifacts";
pub const RUNTIME_MANIFEST_UNKNOWN_JSON_KEY: &str = "unknown";
pub const RUNTIME_MANIFEST_DUPLICATE_ROOT_PREFIX: &str =
    r#"{"schema_version":2,"schema_version":2,"source_revision":""#;
pub const RUNTIME_MANIFEST_ROOT_PREFIX: &str = r#"{"schema_version":2,"source_revision":""#;
pub const RUNTIME_MANIFEST_RUNTIMES_SEPARATOR: &str = r#"","runtimes":"#;
pub const RUNTIME_MANIFEST_ROOT_SUFFIX: &str = "}";
pub const RUNTIME_MANIFEST_ARCHITECTURE_FIELD: &str = r#""architecture":"#;
pub const RUNTIME_MANIFEST_DUPLICATE_ARCHITECTURE_FIELD: &str =
    r#""architecture":"x86_64","architecture":"#;
pub const PASSING_AUDIT_SCRIPT: &[u8] = br##"#!/bin/bash
set -euo pipefail
/usr/bin/shasum -a 256 "$1" | /usr/bin/awk '{print $1}' > "$8"
"##;
pub const REPEATED_ARCHIVE_AUDIT_SCRIPT: &[u8] = br##"#!/bin/bash
set -euo pipefail
before="$(/bin/cat "$1")"
copied="$(/bin/cat "${11}")"
after="$(/bin/cat "${12}")"
test "${before}" = "${copied}"
test "${copied}" = "${after}"
/usr/bin/shasum -a 256 "${13}" | /usr/bin/awk '{print $1}' > "$8"
"##;
pub const REPEATED_MANIFEST_AUDIT_SCRIPT: &[u8] = br##"#!/bin/bash
set -euo pipefail
first="$(/bin/cat "${10}")"
second="$(/bin/cat "${14}")"
test "${first}" = "${second}"
/usr/bin/shasum -a 256 "$1" | /usr/bin/awk '{print $1}' > "$8"
"##;
pub const OVERSIZED_AUDIT_SCRIPT: &[u8] = br##"#!/bin/bash
set -euo pipefail
/usr/bin/printf '%066d' 0 > "$8"
"##;
pub const FIFO_AUDIT_SCRIPT: &[u8] = br##"#!/bin/bash
set -euo pipefail
/usr/bin/mkfifo "$8"
"##;
pub const SWAPPING_AUDIT_SCRIPT: &[u8] = br##"#!/bin/bash
set -euo pipefail
root="$(/bin/pwd)"
/bin/mv "$root" "${root}.moved"
/bin/mkdir "$root"
/usr/bin/shasum -a 256 "$1" | /usr/bin/awk '{print $1}' > "$8"
"##;
pub const PINNED_POLICY_BYTES: &[u8] = b"pinned policy\n";
pub const TRANSIENT_POLICY_SWAP_AUDIT_SCRIPT: &[u8] = br##"#!/bin/bash
set -euo pipefail
root="$(/bin/pwd)"
/bin/mv "$root" "${root}.moved"
/bin/mkdir "$root"
test "$(/bin/cat "$2")" = "pinned policy"
/bin/rmdir "$root"
/bin/mv "${root}.moved" "$root"
/usr/bin/shasum -a 256 "$1" | /usr/bin/awk '{print $1}' > "$8"
"##;
