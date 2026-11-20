use crate::host::tests::*;
const SERVER_WAVE4_TASK_ERROR: &str = "wave4 ingress task failure";
const SERVER_WAVE4_TASK_PANIC: &str = "wave4 ingress task panic";
const SERVER_WAVE4_ACCEPT_ERROR: &str = "wave4 injected accept failure";
const SERVER_WAVE4_SOCKET_NAME: &str = "wave4-ingress.sock";
const SERVER_WAVE4_NO_CLIENT_CAPACITY: usize = 0;
const SERVER_WAVE4_CLIENT_CAPACITY: usize = 1;

#[tokio::test]
async fn host_ingress_shutdown_propagates_task_error_and_panic() {
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let mut failed = HostIngress {
        task: tokio::spawn(async { anyhow::bail!(SERVER_WAVE4_TASK_ERROR) }),
        shutdown: Some(shutdown),
    };
    let error = failed.shutdown().await.unwrap_err();
    assert!(error.to_string().contains(HOST_INGRESS_SHUTDOWN_CONTEXT));
    assert!(format!("{error:#}").contains(SERVER_WAVE4_TASK_ERROR));
    assert!(receiver.await.is_ok());

    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let mut panicked = HostIngress {
        task: tokio::spawn(async {
            panic!("{SERVER_WAVE4_TASK_PANIC}");
            #[allow(unreachable_code)]
            Ok(())
        }),
        shutdown: Some(shutdown),
    };
    let error = panicked.shutdown().await.unwrap_err();
    assert!(error.to_string().contains(HOST_INGRESS_SHUTDOWN_CONTEXT));
    assert!(receiver.await.is_ok());
}

#[tokio::test]
async fn host_ingress_disarm_drops_shutdown_without_signalling() {
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        assert!(receiver.await.is_err());
        Ok(())
    });
    let mut ingress = HostIngress {
        task,
        shutdown: Some(shutdown),
    };
    ingress.disarm();
    ingress.shutdown().await.unwrap();
}

#[tokio::test]
async fn first_ingress_poll_accepts_a_prequeued_client() {
    let directory = tempdir().unwrap();
    let socket = directory.path().join(SERVER_WAVE4_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let warmup_client = UnixStream::connect(&socket).await.unwrap();
    let (warmup_server, _) = listener.accept().await.unwrap();
    drop((warmup_client, warmup_server));
    let client = UnixStream::connect(&socket).await.unwrap();
    let (armed, ready) = tokio::sync::oneshot::channel();

    let accepted = poll_accept_once(&listener, armed).await.unwrap().unwrap();
    ready.await.unwrap();
    drop((accepted, client));
}

#[tokio::test(start_paused = true)]
async fn host_accept_rejects_capacity_backs_off_and_reaps_eof() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let mut tasks = tokio::task::JoinSet::new();
    let mut backoff =
        crate::ingress::AcceptBackoff::new(HOST_ACCEPT_INITIAL_BACKOFF, HOST_ACCEPT_MAX_BACKOFF);

    let clients = Arc::new(tokio::sync::Semaphore::new(SERVER_WAVE4_NO_CLIENT_CAPACITY));
    let (client, server) = UnixStream::pair().unwrap();
    let address = server.peer_addr().unwrap();
    handle_host_accept(
        &host,
        &clients,
        &mut tasks,
        &mut backoff,
        Ok((server, address)),
    )
    .await;
    assert!(tasks.is_empty());
    drop(client);

    let error = std::io::Error::new(
        std::io::ErrorKind::ConnectionAborted,
        SERVER_WAVE4_ACCEPT_ERROR,
    );
    handle_host_accept(&host, &clients, &mut tasks, &mut backoff, Err(error)).await;
    assert_eq!(backoff.failures(), SERVER_WAVE4_CLIENT_CAPACITY as u32);

    let clients = Arc::new(tokio::sync::Semaphore::new(SERVER_WAVE4_CLIENT_CAPACITY));
    let (client, server) = UnixStream::pair().unwrap();
    let address = server.peer_addr().unwrap();
    drop(client);
    handle_host_accept(
        &host,
        &clients,
        &mut tasks,
        &mut backoff,
        Ok((server, address)),
    )
    .await;
    assert!(tasks.join_next().await.unwrap().is_ok());
    assert_eq!(clients.available_permits(), SERVER_WAVE4_CLIENT_CAPACITY);
    assert_eq!(backoff.failures(), SERVER_WAVE4_NO_CLIENT_CAPACITY as u32);
}

#[tokio::test]
async fn ingress_loop_handles_a_prequeued_eof_before_shutdown() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let socket = directory.path().join(SERVER_WAVE4_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let warmup_client = UnixStream::connect(&socket).await.unwrap();
    let (warmup_server, _) = listener.accept().await.unwrap();
    drop((warmup_client, warmup_server));
    let client = UnixStream::connect(&socket).await.unwrap();
    let (armed, ready) = tokio::sync::oneshot::channel();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(run_host_ingress(host, listener, armed, receiver));

    ready.await.unwrap();
    drop(client);
    tokio::task::yield_now().await;
    shutdown.send(()).unwrap();
    task.await.unwrap().unwrap();
}
