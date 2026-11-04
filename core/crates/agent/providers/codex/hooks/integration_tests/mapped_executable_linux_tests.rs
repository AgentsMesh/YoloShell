use super::super::{CodexExecutableAttestation, CodexExecutableKind};

const UNMAPPED_COPY: &str = "unmapped-callback";
const INVALID_PROCESS_PID: u32 = 0;

#[test]
fn actual_linux_process_image_matches_its_retained_executable() {
    let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
    let attestation =
        CodexExecutableAttestation::capture(&executable, CodexExecutableKind::Runtime).unwrap();
    attestation
        .verify_mapped_process(std::process::id(), CodexExecutableKind::Runtime)
        .unwrap();
}

#[test]
fn identical_bytes_at_another_inode_are_not_process_mapping_authority() {
    let root = tempfile::tempdir().unwrap();
    let copy = root.path().canonicalize().unwrap().join(UNMAPPED_COPY);
    std::fs::copy(std::env::current_exe().unwrap(), &copy).unwrap();
    let attestation =
        CodexExecutableAttestation::capture(&copy, CodexExecutableKind::Runtime).unwrap();
    attestation
        .verify_current(CodexExecutableKind::Runtime)
        .unwrap();
    assert!(
        attestation
            .verify_mapped_process(std::process::id(), CodexExecutableKind::Runtime)
            .is_err()
    );
}

#[test]
fn missing_process_cannot_supply_executable_mapping_proof() {
    let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
    let attestation =
        CodexExecutableAttestation::capture(&executable, CodexExecutableKind::Runtime).unwrap();
    assert!(
        attestation
            .verify_mapped_process(INVALID_PROCESS_PID, CodexExecutableKind::Runtime)
            .is_err()
    );
}
