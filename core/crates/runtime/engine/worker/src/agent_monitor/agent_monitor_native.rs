fn synchronize_native_projection(
    state: &WorkerState,
    store: &AgentBindingStore,
    context: &DetectionContext,
    runtime: crate::agent::authority::AuthorityRuntimeIdentity,
) -> Result<Option<AgentBinding>> {
    let raw = store.read_raw_in_namespace(state.namespace.lease())?;
    let canonical =
        store.load_authoritative_in_namespace(state.namespace.lease(), context, runtime)?;
    if raw == canonical {
        return Ok(canonical);
    }
    let expected = lock_recover(&state.record).clone();
    let fence = agent_monitor_publication_fence::acquire(state, &expected)?;
    store.repair_native_binding_in_namespace_with_commit_fence(
        state.namespace.lease(),
        context,
        runtime,
        || fence.verify(),
    )
}
