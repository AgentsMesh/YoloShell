use super::*;
use std::os::unix::fs::symlink;

const BAZEL_RUNTIME_LINK_NAME: &str = "yolo-adversarial";
const RUNTIME_ARTIFACT_NAME: &str = "runtime-artifact";
const RUNTIME_ARTIFACT_BYTES: &[u8] = b"fixture-runtime";

#[test]
fn bazel_style_symlink_resolves_to_regular_runtime_source() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join(RUNTIME_ARTIFACT_NAME);
    let runfile = directory.path().join(BAZEL_RUNTIME_LINK_NAME);
    std::fs::write(&artifact, RUNTIME_ARTIFACT_BYTES).unwrap();
    symlink(&artifact, &runfile).unwrap();

    let resolved = resolve_runtime_binary(runfile);

    assert_eq!(resolved, artifact.canonicalize().unwrap());
    assert!(resolved.symlink_metadata().unwrap().file_type().is_file());
}
