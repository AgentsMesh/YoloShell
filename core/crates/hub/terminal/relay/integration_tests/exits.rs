use super::{constants::*, fixtures::*, peer::*, settings::*};
use tokio::{
    io,
    sync::{mpsc, watch},
    time::timeout_at,
};
use yolo_hub_terminal_protocol::{OutputCursor, TerminalEvent};
use yolo_hub_terminal_relay::{
    ControlIntent, Outcome, RelayError, STATUS_QUEUE_CAPACITY, Status, run,
};

#[tokio::test]
async fn only_origin_exit_events_produce_a_confirmed_exit_and_native_exit_status() {
    let (_directory, socket, listener) = listener();
    let (input, _input_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
    let server = async {
        let (mut control, mut terminal) = observing(&listener).await;
        assert_eq!(
            statuses.recv().await,
            Some(Status::ReadOnly { holder_id: None })
        );
        yolo_hub_terminal_wire::send_event(&mut terminal, &TerminalEvent::Exited {
                    cursor: OutputCursor {
                        target: target(),
                        after_sequence: ZERO_SEQUENCE,
                    },
                    code: Some(EXIT_CODE),
                    signal: None,
                },
                deadline(),
            )
            .await
            .unwrap();
        assert_eq!(statuses.recv().await, Some(Status::SessionExited));
        closed(&mut control).await;
        closed(&mut terminal).await;
    };
    let (result, ()) = timeout_at(deadline(), async {
        tokio::join!(
            run(
                options(socket, ControlIntent::Observe),
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
    assert_eq!(
        result.unwrap(),
        Outcome::Exited {
            code: Some(EXIT_CODE),
            signal: None
        }
    );
}

#[tokio::test]
async fn unreachable_resync_and_stream_loss_are_not_invented_agent_exits() {
    for event in [
        Some(TerminalEvent::Unreachable { target: target() }),
        Some(TerminalEvent::ResyncRequired { target: target() }),
        None,
    ] {
        let (_directory, socket, listener) = listener();
        let (input, _input_peer) = io::duplex(PIPE_CAPACITY);
        let (_resize, sizes) = watch::channel(None);
        let (status, mut statuses) = mpsc::channel(STATUS_QUEUE_CAPACITY);
        let server = async {
            let (mut control, mut terminal) = observing(&listener).await;
            assert_eq!(
                statuses.recv().await,
                Some(Status::ReadOnly { holder_id: None })
            );
            if let Some(event) = &event {
                yolo_hub_terminal_wire::send_event(&mut terminal, &event.clone(),
                        deadline(),
                    )
                    .await
                    .unwrap();
            }
            drop(terminal);
            closed(&mut control).await;
            assert!(statuses.recv().await.is_none());
        };
        let (result, ()) = timeout_at(deadline(), async {
            tokio::join!(
                run(
                    options(socket, ControlIntent::Observe),
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
        match event {
            Some(TerminalEvent::Unreachable { .. }) => {
                assert!(matches!(result, Err(RelayError::Unreachable)))
            }
            Some(TerminalEvent::ResyncRequired { .. }) => {
                assert!(matches!(result, Err(RelayError::ResyncRequired)))
            }
            None => assert!(matches!(result, Err(RelayError::Client(_)))),
            _ => panic!(),
        }
    }
}
