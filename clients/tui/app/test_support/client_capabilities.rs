pub(crate) fn ready(generation: uuid::Uuid) -> yolo_hub_client_channels::ControlEvent {
    yolo_hub_client_channels::ControlEvent::Ready {
        generation,
        hello: yolo_hub_rpc_protocol::Hello {
            hub_id: yolo_hub_identity::HubId::new(),
            version: yolo_hub_rpc_protocol::HUB_PROTOCOL_VERSION,
            methods: yolo_hub_rpc_protocol::SUPPORTED_METHODS.to_vec(),
        },
    }
}

pub(crate) fn state() -> crate::State {
    let mut state = crate::State::default();
    install(&mut state);
    state
}

pub(crate) fn install(state: &mut crate::State) {
    install_without(state, &[]);
}

pub(crate) fn install_without(
    state: &mut crate::State,
    excluded: &[yolo_hub_rpc_protocol::Method],
) {
    let yolo_hub_client_channels::ControlEvent::Ready { hello, .. } = ready(uuid::Uuid::new_v4())
    else {
        unreachable!()
    };
    let mut hello = hello;
    hello
        .methods
        .extend_from_slice(yolo_hub_rpc_protocol::MANAGEMENT_METHODS);
    hello.methods.retain(|method| !excluded.contains(method));
    assert!(crate::client_capabilities::install(state, &hello));
}
