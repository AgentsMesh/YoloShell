use super::*;

#[test]
fn current_probe_identity_is_reused_for_its_eof_publication() {
    let probe = KernelPtyEofIdentity::from_kernel_read();

    let selected = publication_identity(Some(Arc::clone(&probe)), Some(&probe));

    assert!(Arc::ptr_eq(&selected, &probe));
}

#[test]
fn stale_probe_identity_is_not_reused_for_a_new_eof() {
    let stale = KernelPtyEofIdentity::from_kernel_read();

    let selected = publication_identity(Some(Arc::clone(&stale)), None);

    assert!(!Arc::ptr_eq(&selected, &stale));
}
