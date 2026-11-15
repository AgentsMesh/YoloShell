fn live_connection(state: &ConnectionState, retired: u64) -> Option<ObservedConnection> {
    if state.generation <= retired {
        return None;
    }
    state
        .connection
        .as_ref()
        .map(|connection| observed(Arc::clone(connection), state.generation))
}

fn observed(connection: Arc<dyn MultiplexedSshConnection>, generation: u64) -> ObservedConnection {
    ObservedConnection {
        id: connection.info().connection_id,
        connection,
        generation,
    }
}
