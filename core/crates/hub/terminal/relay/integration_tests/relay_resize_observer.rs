#[tokio::test]
async fn resize_completion_delivers_a_size_that_arrived_while_the_call_was_in_flight() {
    let (_directory, socket, listener) = listener();
    let mut options = options(socket, ControlIntent::Acquire);
    options.size = None;
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (resize, sizes) = watch::channel(None);
    let (status, mut statuses) = mpsc::channel(STATUS_CAPACITY);
    let grant = grant();
    let first = TerminalSize::new(100, 40).unwrap();
    let second = TerminalSize::new(120, 50).unwrap();
    let server = async {
        let (mut control, mut terminal) = acquiring(&listener, grant).await;
        assert_eq!(statuses.recv().await, Some(Status::ControlAcquired));
        resize.send(Some(first)).unwrap();
        let initial = receive(&mut control).await;
        assert!(matches!(
            initial.fields().command,
            Command::Resize { request } if request.size == first
        ));
        resize.send(Some(second)).unwrap();
        respond(&mut control, &initial, Reply::Completed {}).await;
        let coalesced = receive(&mut control).await;
        assert!(
            matches!(
                coalesced.fields().command,
                Command::Resize { request } if request.size == second
            ),
            "the final size of a drag must reach the Hub after the first call completes"
        );
        respond(&mut control, &coalesced, Reply::Completed {}).await;
        input_peer.shutdown().await.unwrap();
        // Detach may race with the completed resize returning its client, so
        // the release can arrive on the warm channel or a fresh one.
        let (mut release_peer, release) = match control
            .receive::<yolo_hub_jsonrpc_protocol::Call>(deadline())
            .await
        {
            Ok(Incoming::Message(request)) => (control, request),
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
async fn observer_discards_input_without_acquiring_resizing_or_releasing_authority() {
    let (_directory, socket, listener) = listener();
    let options = options(socket, ControlIntent::Observe);
    let (input, mut input_peer) = io::duplex(PIPE_CAPACITY);
    let (_resize, sizes) = watch::channel(options.size);
    let (status, mut statuses) = mpsc::channel(STATUS_CAPACITY);
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
            Reply::LeaseObservation { lease: None },
        )
        .await;
        let mut terminal = attach(&listener).await;
        assert_eq!(
            statuses.recv().await,
            Some(Status::ReadOnly { holder_id: None })
        );
        input_peer.write_all(INPUT).await.unwrap();
        input_peer.shutdown().await.unwrap();
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
