#[cfg(target_os = "macos")]
fn armed_custody_for_active_predecessor(
    fixture: &Fixture,
    record: &RuntimeRecord,
    holder: crate::pty_custody::CustodyHolderIdentity,
) -> crate::pty_custody::CustodyDurableRecord {
    let draft_identity = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap()
        .with_holder_for_test(holder);
    let durable = armed_custody_with_identity_for_test(record, draft_identity);
    let identity = secondary_custody_authority::ReadyWorkerCustodyIdentity::from_durable_for_test(
        durable.armed_lease().unwrap(),
    );
    fixture
        .state
        .secondary_custody_authority
        .replace_ready_for_test(identity);
    durable
}
