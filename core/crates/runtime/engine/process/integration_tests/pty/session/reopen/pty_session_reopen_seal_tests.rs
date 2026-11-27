use super::*;

const DISCOVERY_CAPABILITY: &str = "reopen-seal-discovery-test";
#[cfg(target_os = "linux")]
const TEST_LINUX_DEVICE: u64 = 1;

#[cfg(not(target_os = "macos"))]
#[test]
fn unsupported_platform_commit_is_a_noop_for_each_proof_scope() {
    let mut discovery = PtySessionProof::discovery(DISCOVERY_CAPABILITY.to_owned()).unwrap();
    let authority = discovery.acquire_reopen_seal().unwrap();
    let committed = authority.committed_identity();
    discovery.install_reopen_seal(authority).unwrap();
    discovery.commit_reopen_seal_for_destruction().unwrap();
    discovery
        .revalidate_committed_reopen_seal(&committed)
        .unwrap();

    #[cfg(target_os = "linux")]
    {
        let mut pty =
            PtySessionProof::restore_linux(TEST_LINUX_DEVICE, DISCOVERY_CAPABILITY.to_owned())
                .unwrap();
        let authority = pty.acquire_reopen_seal().unwrap();
        let committed = authority.committed_identity();
        pty.install_reopen_seal(authority).unwrap();
        pty.commit_reopen_seal_for_destruction().unwrap();
        pty.revalidate_committed_reopen_seal(&committed).unwrap();
    }
}

#[test]
fn discovery_proof_accepts_only_its_typed_not_applicable_authority() {
    let mut proof = PtySessionProof::discovery(DISCOVERY_CAPABILITY.to_owned()).unwrap();
    let authority = proof.acquire_reopen_seal().unwrap();
    let committed = authority.committed_identity();
    proof.install_reopen_seal(authority).unwrap();
    proof.commit_reopen_seal_for_destruction().unwrap();
    proof.revalidate_committed_reopen_seal(&committed).unwrap();
    let missing = PtyReopenSealAuthority::missing_proof();
    let missing_committed = missing.committed_identity();

    assert!(proof.install_reopen_seal(missing).is_err());
    assert!(
        proof
            .revalidate_committed_reopen_seal(&missing_committed)
            .is_err()
    );
}

#[cfg(target_os = "macos")]
#[test]
fn production_pty_proof_rejects_missing_proof_authority() {
    use portable_pty::{PtySize, native_pty_system};
    const TERMINAL_ROWS: u16 = 24;
    const TERMINAL_COLUMNS: u16 = 80;
    const TERMINAL_PIXEL_SIZE: u16 = 0;
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLUMNS,
            pixel_width: TERMINAL_PIXEL_SIZE,
            pixel_height: TERMINAL_PIXEL_SIZE,
        })
        .unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), DISCOVERY_CAPABILITY.to_owned())
            .unwrap();

    assert!(
        proof
            .install_reopen_seal(PtyReopenSealAuthority::missing_proof())
            .is_err()
    );
}
