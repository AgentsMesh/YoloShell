const OUTPUT_TEST_SEQUENCE: u64 = 1;

struct FailingIo;

impl Read for FailingIo {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
}

impl Write for FailingIo {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
}

async fn authenticated_worker_connection(
    fixture: &Fixture,
    generation: u64,
) -> (WorkerConnection, tokio::task::JoinHandle<Result<()>>) {
    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });
    let auth = RequestEnvelope::new(
        "auth",
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: "test-token".into(),
            controller_generation: generation,
        })
        .unwrap(),
    );
    write_frame(&mut client, &auth).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    assert!(response.ok);
    (WorkerConnection::open(client).await.unwrap(), task)
}

fn begin_worker_request(mut client: WorkerConnection, request: RequestEnvelope)
    -> tokio::task::JoinHandle<(WorkerConnection, std::result::Result<ResponseEnvelope, WorkerRpcError>)> {
    tokio::spawn(async move { let response = client.exchange(&request).await; (client, response) })
}

async fn drive_rpc<T>(future: impl std::future::Future<Output = T>) -> T {
    const SCHEDULING_BUDGET: Duration = Duration::from_secs(5);
    const SCHEDULING_TIMEOUT: &str = "RPC executor did not settle within the test scheduling budget";
    let limit = std::time::Instant::now() + SCHEDULING_BUDGET;
    tokio::pin!(future);
    loop {
        assert!(std::time::Instant::now() < limit, "{SCHEDULING_TIMEOUT}");
        tokio::select! { biased; value = &mut future => return value, _ = tokio::task::yield_now() => {} }
    }
}
