#[path = "constants.rs"]
pub mod constants;
pub use constants::*;
use uuid::Uuid;
use yolo_hub_client_access_protocol::*;
use yolo_hub_client_access_registry::{AccessRegistry, ClientAdmission};
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_routing::{Right, Rights};
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::*;

pub fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
pub fn hub(value: u128) -> HubId {
    HubId::from_uuid(id(value)).unwrap()
}
pub fn address(origin: u128, session: u128) -> TerminalAddress {
    TerminalAddress {
        hub_id: hub(origin),
        terminal_id: TerminalId::from_uuid(id(session)).unwrap(),
    }
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
pub fn configuration(rights: &[Right]) -> AccessConfiguration {
    AccessConfiguration {
        schema_version: ACCESS_SCHEMA_VERSION,
        entry_hub: hub(ENTRY),
        revision: FIRST_ACCESS_REVISION,
        clients: vec![ClientGrant {
            identity: ClientIdentity {
                client_id: id(CLIENT),
                public_key: [KEY_BYTE; ED25519_PUBLIC_KEY_BYTES],
            },
            origins: vec![OriginAccess {
                origin: hub(ORIGIN),
                rights: rights.iter().copied().collect::<Rights>(),
            }],
            expires_at_ms: now() + GRANT_LIFETIME_MS,
        }],
    }
}
pub fn authority(rights: &[Right]) -> (AccessRegistry, ClientAdmission) {
    let registry = AccessRegistry::new(configuration(rights)).unwrap();
    let admission = registry
        .admit_verified_key(&[KEY_BYTE; ED25519_PUBLIC_KEY_BYTES], now())
        .unwrap();
    (registry, admission)
}
pub fn request(destination: u128, command: Command) -> Request {
    Request::new(hub(destination), command).unwrap()
}
pub fn summary(origin: u128, session: u128) -> TerminalSummary {
    TerminalSummary::new(TerminalSummaryFields {
        presentation: Default::default(),
        address: address(origin, session),
        name: NAME.into(),
        cwd: DIRECTORY.into(),
        archived: false,
        recovery: None,
        runtime: None,
        agent: None,
        workspace: None,
    })
    .unwrap()
}
pub fn target() -> RuntimeTarget {
    RuntimeTarget {
        address: address(ORIGIN, SESSION),
        runtime_epoch: RuntimeEpoch::from_uuid(id(RUNTIME)).unwrap(),
    }
}
pub fn principal(client: u128) -> WriterPrincipal {
    WriterPrincipal {
        client_id: id(client),
        controller_instance_id: id(CONTROLLER),
    }
}
pub fn grant(client: u128) -> LeaseGrant {
    LeaseGrant::new(LeaseGrantFields {
        target: target(),
        principal: principal(client),
        generation: FIRST_LEASE_GENERATION,
        capability: SecretCapability::from_uuid(Uuid::new_v4()).unwrap(),
        expires_at_ms: LEASE_EXPIRY,
        ttl_ms: LEASE_TTL,
        renew_after_ms: LEASE_RENEW,
    })
    .unwrap()
}
pub fn writer_commands(client: u128) -> Vec<Command> {
    let grant = grant(client);
    vec![
        Command::Lease {
            request: LeaseRequest {
                request_id: id(REQUEST),
                target: target(),
                principal: principal(client),
                mode: LeaseMode::Acquire {},
            },
        },
        Command::Renew { grant },
        Command::Release { grant },
        Command::Input {
            request: InputRequest::new(InputFields {
                request_id: id(REQUEST),
                grant,
                data: INPUT.to_vec(),
            })
            .unwrap(),
        },
        Command::Resize {
            request: ResizeRequest {
                request_id: id(REQUEST),
                grant,
                size: TerminalSize::default(),
            },
        },
    ]
}
pub fn create() -> CreateShellRequest {
    CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: id(REQUEST),
        creation_token: Uuid::new_v4(),
        name: Some(NAME.into()),
        cwd: DIRECTORY.into(),
        size: TerminalSize::default(),
    })
    .unwrap()
}
