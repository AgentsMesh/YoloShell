use super::{constants::*, fixture::*, host::Host};
use yolo_hub_connector_jsonrpc::{Answer, Call};
use yolo_hub_connector_protocol::{Activation, Command, Reply, Request};
use yolo_hub_identity::ConnectorInstanceId;
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_local_unix::LocalUnixStream;

pub type Channel = Framed<LocalUnixStream>;

pub async fn call(channel: &mut Channel, request: Request) -> Reply {
    let request = Call::from_request(request);
    channel.send(&request, deadline()).await.unwrap();
    let Incoming::Message(response) = channel.receive::<Answer>(deadline()).await.unwrap() else {
        panic!("{UNEXPECTED_REPLY}");
    };
    response.reply(&request).unwrap()
}

pub async fn enroll_and_publish(host: &Host, channel: &mut Channel) -> Activation {
    assert_eq!(
        call(channel, host.fixture.request(Command::Hello {})).await,
        Reply::Hello {
            hub: host.fixture.state.hub_id()
        }
    );
    let registered = host
        .respond(call(
            channel,
            host.fixture.request(Command::Register {
                scope: host.scope.clone(),
            }),
        ))
        .await;
    let Reply::Enrolled { enrollment } = registered else {
        panic!("{UNEXPECTED_REPLY}");
    };
    let active = host
        .respond(call(
            channel,
            host.fixture.request(Command::Activate {
                connector_id: enrollment.connector_id,
                credential: enrollment.credential,
                instance_id: ConnectorInstanceId::new(),
            }),
        ))
        .await;
    let Reply::Activated { activation } = active else {
        panic!("{UNEXPECTED_REPLY}");
    };
    let published = host
        .respond(call(
            channel,
            host.fixture.request(Command::Publish {
                publication: publication(activation),
            }),
        ))
        .await;
    let Reply::Published { ack } = published else {
        panic!("{UNEXPECTED_REPLY}");
    };
    assert_eq!(ack.connector_id, activation.connector_id);
    assert_eq!(ack.sequence, activation.next_sequence);
    activation
}
