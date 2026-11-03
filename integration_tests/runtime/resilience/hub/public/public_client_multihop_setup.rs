use crate::hub::federation_support;
use yolo_hub_client::EndpointClient;

const TERMINAL_LINE_END: u8 = b'\r';
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::{HubId, RuntimeTarget, TerminalAddress};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, InputFields, InputRequest, LeaseGrant, LeaseMode,
    LeaseRequest, TERMINAL_PROTOCOL_SCHEMA_VERSION, TerminalSize, WriterPrincipal,
};

pub fn configuration(
    a: yolo_hub_delegation::PeerIdentity,
    b: yolo_hub_delegation::PeerIdentity,
    c: yolo_hub_delegation::PeerIdentity,
    addresses: (
        std::net::SocketAddr,
        std::net::SocketAddr,
        std::net::SocketAddr,
    ),
) -> (Configuration, Configuration, Configuration) {
    let (a_addr, b_addr, c_addr) = addresses;
    (
        Configuration {
            listen: Some(a_addr),
            trusted_origins: vec![c],
            file_roots: vec![],
            peers: vec![federation_support::peer(b, Some(b_addr), true, vec![])],
        },
        Configuration {
            listen: Some(b_addr),
            trusted_origins: vec![],
            file_roots: vec![],
            peers: vec![
                federation_support::peer(
                    a,
                    None,
                    false,
                    vec![
                        federation_support::exports(b.hub(), false),
                        federation_support::exports(c.hub(), false),
                    ],
                ),
                federation_support::peer(c, None, true, vec![]),
            ],
        },
        Configuration {
            listen: Some(c_addr),
            trusted_origins: vec![],
            file_roots: vec![],
            peers: vec![federation_support::peer(
                b,
                Some(b_addr),
                false,
                vec![federation_support::exports(c.hub(), true)],
            )],
        },
    )
}

pub async fn create_shell(
    client: &mut EndpointClient,
    hub: HubId,
    host: &crate::Harness,
) -> (TerminalAddress, RuntimeTarget) {
    let token = uuid::Uuid::new_v4();
    let request = CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        creation_token: token,
        name: Some(super::super::constants::MULTIHOP_SHELL_NAME.into()),
        cwd: host.home.path().display().to_string(),
        size: TerminalSize::default(),
    })
    .unwrap();
    let Reply::Created { creation } = client
        .call(
            hub,
            Command::Create { request },
            federation_support::deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("remote shell creation failed")
    };
    let view = host
        .client()
        .get_session(creation.address.terminal_id.as_uuid())
        .await
        .unwrap();
    host.track_view(&view);
    client
        .call(
            hub,
            Command::Finalize {
                creation_token: token,
                creation,
            },
            federation_support::deadline(),
        )
        .await
        .unwrap();
    let Reply::Terminal { terminal: session } = client
        .call(
            hub,
            Command::Get {
                address: creation.address,
            },
            federation_support::deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("remote shell finalization failed")
    };
    (creation.address, session.fields().runtime.unwrap().target)
}

pub async fn acquire(
    client: &mut EndpointClient,
    target: RuntimeTarget,
    client_id: uuid::Uuid,
) -> LeaseGrant {
    let Reply::Lease { grant } = client
        .call(
            target.address.hub_id,
            Command::Lease {
                request: LeaseRequest {
                    request_id: uuid::Uuid::new_v4(),
                    target,
                    principal: WriterPrincipal {
                        client_id,
                        controller_instance_id: uuid::Uuid::new_v4(),
                    },
                    mode: LeaseMode::Acquire {},
                },
            },
            federation_support::deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("multihop writer acquisition failed")
    };
    grant
}

pub fn input(grant: LeaseGrant, data: &[u8]) -> Command {
    Command::Input {
        request: InputRequest::new(InputFields {
            request_id: uuid::Uuid::new_v4(),
            grant,
            data: data.to_vec(),
        })
        .unwrap(),
    }
}

pub fn marker() -> (Vec<u8>, String) {
    let nonce = uuid::Uuid::new_v4();
    let prefix = super::super::constants::MULTIHOP_MARKER_PREFIX;
    let mut command = format!("printf '{prefix}%s\\n' {nonce}").into_bytes();
    command.push(TERMINAL_LINE_END);
    (command, format!("{prefix}{nonce}"))
}
