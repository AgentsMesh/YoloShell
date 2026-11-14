use super::*;
use std::sync::atomic::AtomicUsize;

#[test]
fn invalidated_eof_waits_for_a_fresh_complete_kernel_read() {
    let mut output = ExclusivePtyOutput::quiescent_for_test(Arc::new(AtomicUsize::default()));
    output.record_kernel_eof_progress(OutputProgressState::Complete);
    let stale = output.kernel_eof_identity_for_test().unwrap();

    output.revoke_kernel_eof_authority();
    output.record_kernel_eof_progress(OutputProgressState::Quiescent);
    assert!(output.kernel_eof_identity_for_test().is_none());

    output.record_kernel_eof_progress(OutputProgressState::Complete);
    let fresh = output.kernel_eof_identity_for_test().unwrap();
    assert!(!Arc::ptr_eq(&stale, &fresh));
}
