use super::{constants::*, fixtures::*, peer::*, settings::*};
use tokio::{
    io::{self, AsyncWriteExt},
    sync::{mpsc, watch},
    time::timeout_at,
};
use yolo_hub_client::ClientError;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseMode, PublicError};
use yolo_hub_terminal_relay::{
    ControlIntent, Outcome, RelayError, STATUS_QUEUE_CAPACITY, Status, run,
};

#[tokio::test]
async fn stale_takeover_is_not_retried_after_the_observed_generation_changes() {
    let (_directory, socket, listener) = listener();
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let server = async {
        let mut control = accept(&listener).await;
        let request = receive(&mut control).await;
        assert_eq!(
            request.fields().command,
            Command::ObserveLease { target: target() }
        );
        respond(
            &mut control,
            &request,
            Reply::LeaseObservation {
                lease: Some(observation(GENERATION)),
            },
        )
        .await;
        let request = receive(&mut control).await;
        assert!(
            matches!(request.fields().command, Command::Lease { request } if request.mode == LeaseMode::TakeControl { observed_generation: GENERATION })
        );
        respond(
            &mut control,
            &request,
            Reply::Error {
                code: PublicError::Conflict,
            },
        )
        .await;
        let request = receive(&mut control).await;
        assert_eq!(
            request.fields().command,
            Command::ObserveLease { target: target() }
        );
        respond(
            &mut control,
            &request,
            Reply::LeaseObservation {
                lease: Some(observation(GENERATION + GENERATION)),
            },
        )
        .await;
        let mut terminal = attach(&listener).await;
        assert_eq!(
            statuses.recv().await,
            Some(Status::ReadOnly {
                holder_id: Some(OTHER_HOLDER.into())
            })
        );
        input_peer.write_all(INPUT).await.unwrap();
        input_peer.shutdown().await.unwrap();
        closed(&mut control).await;
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run(
                options(socket, ControlIntent::TakeControl),
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
async fn revoked_renewal_retires_writer_and_never_reacquires_or_replays_buffered_input() {
    let (_directory, socket, listener) = listener();
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let grant = grant();
    let server = async {
        let (mut control, mut terminal) = acquiring(&listener, grant).await;
        assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
        let renewal = receive(&mut control).await;
        assert_eq!(renewal.fields().command, Command::Renew { grant });
        respond(
            &mut control,
            &renewal,
            Reply::Error {
                code: PublicError::Conflict,
            },
        )
        .await;
        assert_eq!(
            statuses.recv().await,
            Some(Status::ControlLost { holder_id: None })
        );
        input_peer.write_all(INPUT).await.unwrap();
        input_peer.shutdown().await.unwrap();
        let release = receive(&mut control).await;
        assert_eq!(release.fields().command, Command::Release { grant });
        respond(
            &mut control,
            &release,
            Reply::Error {
                code: PublicError::Conflict,
            },
        )
        .await;
        closed(&mut control).await;
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run(
                options(socket, ControlIntent::Acquire),
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
async fn lost_input_response_only_attempts_exact_grant_release_on_a_fresh_channel() {
    let (_directory, socket, listener) = listener();
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let grant = grant();
    let server = async {
        let (mut control, mut terminal) = acquiring(&listener, grant).await;
        assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
        input_peer.write_all(INPUT).await.unwrap();
        let input = receive(&mut control).await;
        assert!(matches!(input.fields().command, Command::Input { .. }));
        drop(control);
        let mut release_channel = accept(&listener).await;
        let release = receive(&mut release_channel).await;
        assert_eq!(release.fields().command, Command::Release { grant });
        respond(&mut release_channel, &release, Reply::Completed {}).await;
        closed(&mut release_channel).await;
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run(
                options(socket, ControlIntent::Acquire),
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
    assert!(matches!(
        result,
        Err(RelayError::Client(ClientError::OutcomeUnknown(_)))
    ));
}
