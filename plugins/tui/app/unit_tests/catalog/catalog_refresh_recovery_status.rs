#[test]
fn a_catalogue_failure_does_not_hide_a_newer_transport_failure_from_recovery() {
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    let request_id = Uuid::new_v4();
    state.catalog_pending = Some(request_id);
    catalog_events::apply(
        &mut state,
        CatalogEvent {
            request_id,
            result: Err(CatalogError::Limit),
        },
    );
    let message = "Hub transport failed: Hub did not respond in time; retry shortly";
    let (jobs, _incoming) = mpsc::channel(QUEUE_CAPACITY);
    crate::events::apply(
        &mut state,
        &jobs,
        crate::network::Update::Failed {
            command: None,
            message: message.into(),
            connected: true,
            transient: true,
        },
    );
    assert_eq!(state.status, message);

    let recovered_id = Uuid::new_v4();
    state.catalog_pending = Some(recovered_id);
    catalog_events::apply(
        &mut state,
        CatalogEvent {
            request_id: recovered_id,
            result: Ok(Catalog::default()),
        },
    );
    assert_ne!(state.status, message);
    assert_eq!(state.recoverable_status, None);
}
