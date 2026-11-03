use super::{constants::*, fixtures::*, peer::*};
use tokio::{
    io::{self, AsyncWriteExt},
    sync::{mpsc, watch},
    time::timeout_at,
};
use yolo_hub_jsonrpc_transport::Incoming;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::TerminalSize;
use yolo_hub_terminal_relay::{ControlIntent, Options, Outcome, Status, run};

fn options(socket: std::path::PathBuf, intent: ControlIntent) -> Options {
    Options {
        socket,
        entry: hub(),
        target: target(),
        principal: principal(),
        intent,
        size: Some(TerminalSize::default()),
    }
}

const STATUS_CAPACITY: usize = yolo_hub_terminal_relay::STATUS_QUEUE_CAPACITY;

#[tokio::test]
async fn stalled_input_does_not_block_lease_renewal_on_a_separate_lane() {
    let (_directory, socket, listener) = listener();
    let mut options = options(socket, ControlIntent::Acquire);
    options.size = None;
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_CAPACITY);
    let grant = grant();
    let server = async {
        let (mut control, mut terminal) = acquiring(&listener, grant).await;
        assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
        input_peer.write_all(INPUT).await.unwrap();
        let input_request = receive(&mut control).await;
        assert!(matches!(
            input_request.fields().command,
            Command::Input { .. }
        ));
        let mut renewal_channel = accept(&listener).await;
        let renewal = receive(&mut renewal_channel).await;
        assert_eq!(renewal.fields().command, Command::Renew { grant });
        respond(&mut renewal_channel, &renewal, Reply::Lease { grant }).await;
        assert_eq!(statuses.try_recv(), Err(mpsc::error::TryRecvError::Empty));
        respond(
            &mut control,
            &input_request,
            Reply::Input { duplicate: false },
        )
        .await;
        input_peer.shutdown().await.unwrap();
        let release = receive(&mut control).await;
        assert_eq!(release.fields().command, Command::Release { grant });
        respond(&mut control, &release, Reply::Completed {}).await;
        closed(&mut control).await;
        closed(&mut renewal_channel).await;
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run(
                options,
                input,
                io::sink(),
                sizes,
                status,
                std::future::pending()
            ),
            server,
        )
    })
    .await
    .unwrap();
    assert_eq!(result.unwrap(), Outcome::Detached);
}

#[tokio::test]
async fn input_reconnects_when_renewal_temporarily_owns_the_only_client() {
    let (_directory, socket, listener) = listener();
    let mut options = options(socket, ControlIntent::Acquire);
    options.size = None;
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_CAPACITY);
    let grant = grant();
    let server = async {
        let (mut control, mut terminal) = acquiring(&listener, grant).await;
        assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
        // With no input in flight, renewal borrows the warm control channel.
        let renewal = receive(&mut control).await;
        assert_eq!(renewal.fields().command, Command::Renew { grant });
        input_peer.write_all(INPUT).await.unwrap();
        let mut input_channel = accept(&listener).await;
        let request = receive(&mut input_channel).await;
        assert!(matches!(request.fields().command, Command::Input { .. }));
        respond(&mut control, &renewal, Reply::Lease { grant }).await;
        respond(
            &mut input_channel,
            &request,
            Reply::Input { duplicate: false },
        )
        .await;
        input_peer.shutdown().await.unwrap();
        let (mut release_peer, release) = match input_channel
            .receive::<yolo_hub_jsonrpc_protocol::Call>(deadline())
            .await
        {
            Ok(Incoming::Message(request)) => (input_channel, request),
            Ok(Incoming::Eof) => {
                let mut fresh = accept(&listener).await;
                let request = receive(&mut fresh).await;
                (fresh, request)
            }
            _ => panic!("relay closed without releasing its lease"),
        };
        assert_eq!(release.fields().command, Command::Release { grant });
        respond(&mut release_peer, &release, Reply::Completed {}).await;
        closed(&mut release_peer).await;
        closed(&mut control).await;
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run(
                options,
                input,
                io::sink(),
                sizes,
                status,
                std::future::pending()
            ),
            server,
        )
    })
    .await
    .unwrap();
    assert_eq!(result.unwrap(), Outcome::Detached);
}


include!("relay_resize_observer.rs");
