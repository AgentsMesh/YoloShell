fn assert_exact_binding_and_fail_closed_recovery(
    store: &AgentBindingStore,
    context: &DetectionContext,
    expected: &AgentBinding,
) {
    let raw: AgentBinding = crate::paths::read_json(store.path()).unwrap();
    assert_eq!(&raw, expected);

    let mut fail_closed = expected.clone();
    fail_closed.state = crate::agent::AgentState::Unknown;
    fail_closed.recovery_availability = crate::agent::AgentRecoveryAvailability::Unavailable;
    assert_eq!(store.load(context).unwrap(), Some(fail_closed));
}
