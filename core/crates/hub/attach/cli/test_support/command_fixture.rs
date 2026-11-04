use std::os::fd::IntoRawFd;
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    FIRST_OBSERVATION_SEQUENCE, ObservationCursor,
};
use yolo_hub_identity::{
    AgentAddress, AgentId, AgentTarget, ExecutionId, HubId, RuntimeEpoch, RuntimeTarget,
    TerminalAddress, TerminalId,
};
use yolo_hub_relay_control_protocol::{Request, RequestFields, SCHEMA_VERSION};

pub fn channel(
    request: &Request,
) -> (
    crate::command_io::CommandIo,
    yolo_hub_terminal_relay::CommandChannel,
    tokio::net::UnixStream,
) {
    let (child, parent) = std::os::unix::net::UnixStream::pair().unwrap();
    parent.set_nonblocking(true).unwrap();
    let target = request.fields().terminal;
    let args = crate::Arguments {
        socket: super::constants::SOCKET.into(),
        hub: target.address.hub_id,
        origin: target.address.hub_id,
        terminal_id: target.address.terminal_id,
        runtime_epoch: target.runtime_epoch,
        holder_id: Uuid::new_v4(),
        status_fd: None,
        command_fd: Some(child.into_raw_fd()),
        launch_id: Some(request.fields().launch_id),
        observe: true,
        take_control: false,
    };
    let (io, incoming) = crate::command_io::CommandIo::adopt(&args).unwrap().unwrap();
    (
        io,
        incoming,
        tokio::net::UnixStream::from_std(parent).unwrap(),
    )
}

pub fn request() -> Request {
    let hub_id = HubId::new();
    let target = AgentTarget {
        address: AgentAddress {
            hub_id,
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    };
    Request::new(RequestFields {
        schema_version: SCHEMA_VERSION,
        launch_id: Uuid::new_v4(),
        terminal: RuntimeTarget {
            address: TerminalAddress {
                hub_id,
                terminal_id: TerminalId::new(),
            },
            runtime_epoch: RuntimeEpoch::new(),
        },
        request: AgentControlRequest::new(AgentControlFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            request_id: Uuid::new_v4(),
            target,
            expected_cursor: ObservationCursor::new(
                target,
                Uuid::new_v4(),
                FIRST_OBSERVATION_SEQUENCE,
            )
            .unwrap(),
            action: AgentAction::Interrupt {},
        })
        .unwrap(),
    })
    .unwrap()
}
