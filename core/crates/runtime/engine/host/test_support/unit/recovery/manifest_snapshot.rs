use super::*;

pub(super) fn manifest_snapshot(manifest: &TerminalManifest) -> Vec<u8> {
    serde_json::to_vec(manifest).unwrap()
}

pub(super) fn assert_manifest_snapshot(manifest: &TerminalManifest, expected: &[u8]) {
    assert_eq!(manifest_snapshot(manifest), expected);
}
