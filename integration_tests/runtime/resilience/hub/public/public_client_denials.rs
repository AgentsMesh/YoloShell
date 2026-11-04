use super::{deadline, fixture::Fixture};
use yolo_hub_client::HubEntryStream;
use yolo_hub_client_access_protocol::{ACCESS_REVISION_STEP, ClientAccessChange};
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, Method, Reply};
use yolo_hub_terminal_protocol::{AttachRequest, LeaseMode, PublicError};

#[tokio::test(flavor = "multi_thread")]
async fn openssh_observe_only_rejects_forged_mutations_at_the_production_hub() {
    let fixture = Fixture::start(&[Right::Observe]).await;
    let (session, before) = fixture.create().await;
    let target = session.fields().runtime.unwrap().target;
    let entry = fixture.entry();
    let mut client = fixture.connect(&entry).await;
    for forbidden in [
        Method::Create,
        Method::Lease,
        Method::ConfigureClientAccess,
        Method::Attach,
    ] {
        assert!(!client.hello().methods.contains(&forbidden));
    }
    let Reply::Terminals {
        terminals,
        next: None,
    } = client
        .call(
            fixture.hub,
            Command::List {
                include_archived: false,
                after: None,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("missing public session catalogue")
    };
    assert_eq!(terminals.len(), super::constants::SHELL_COUNT);
    assert_eq!(terminals.first().unwrap().fields().address, target.address);
    // Bypass Client's Hello preflight: rejection must happen at the Hub itself.
    let mut wire = client.into_channel().unwrap();
    let current = fixture.configuration().await;
    let mut proposed = current.clone();
    proposed.revision += ACCESS_REVISION_STEP;
    proposed.clients.clear();
    let commands = [
        Command::Create {
            request: fixture.create_request(),
        },
        fixture.lease_command(target, LeaseMode::Acquire {}),
        Command::Attach {
            request: AttachRequest::new(target, None).unwrap(),
        },
        Command::GetClientAccess {},
        Command::ConfigureClientAccess {
            change: ClientAccessChange {
                request_id: uuid::Uuid::new_v4(),
                expected_revision: current.revision,
                configuration: proposed,
            },
        },
    ];
    for command in commands {
        denied(&mut wire, fixture.hub, command).await;
    }
    assert_eq!(fixture.configuration().await, current);
    fixture.no_writer(target).await;
    assert_eq!(
        fixture
            .host
            .client()
            .list_sessions_complete(false)
            .await
            .unwrap()
            .len(),
        super::constants::SHELL_COUNT
    );
    fixture.custody(&before).await;
    drop(wire);
    entry.close(deadline()).await.unwrap();
    fixture.stop().await;
}

async fn denied(
    wire: &mut Framed<HubEntryStream>,
    hub: yolo_hub_identity::HubId,
    command: Command,
) {
    let request = Call::new(hub, command).unwrap();
    wire.send(&request, deadline()).await.unwrap();
    let Incoming::Message(response): Incoming<Answer> = wire.receive(deadline()).await.unwrap()
    else {
        panic!("rejected command lost its correlated response")
    };
    assert_eq!(
        response.reply(&request).unwrap(),
        Reply::Error {
            code: PublicError::Unauthorized
        }
    );
}
