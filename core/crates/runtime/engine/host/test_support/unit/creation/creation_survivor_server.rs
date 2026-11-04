const RETRIED_TERMINATION_REQUESTS: usize = 2;
const CLEANUP_SERVER_DEADLINE: &str = "cleanup worker fixture did not finish its requests";
const CLEANUP_REQUEST_DEADLINE: &str = "cleanup request did not complete";

struct CleanupTerminationServer {
    requests: Arc<std::sync::atomic::AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl CleanupTerminationServer {
    fn start(fixture: &SurvivorCreationFixture) -> Self {
        let listener = bind_private_cleanup_listener(&fixture.runtime.socket_path);
        let requests = Arc::new(std::sync::atomic::AtomicUsize::new(usize::default()));
        let task = tokio::spawn(serve_cleanup_terminations(
            listener,
            fixture.runtime.clone(),
            survivor_error(),
            Arc::clone(&requests),
            RETRIED_TERMINATION_REQUESTS,
        ));
        Self { requests, task }
    }

    async fn request(
        &self,
        fixture: &SurvivorCreationFixture,
    ) -> std::result::Result<CleanupTerminationOutcome, ErrorBody> {
        let before = self.requests.load(std::sync::atomic::Ordering::SeqCst);
        let outcome = tokio::time::timeout(
            HOST_REQUEST_TIMEOUT,
            fixture
                .host
                .cleanup_terminate_session(cleanup_termination_request(&fixture.binding)),
        )
        .await
        .expect(CLEANUP_REQUEST_DEADLINE);
        assert_eq!(
            self.requests.load(std::sync::atomic::Ordering::SeqCst),
            before + SINGLE_TERMINATION_REQUEST
        );
        outcome
    }

    async fn finish(&mut self) {
        tokio::time::timeout(HOST_REQUEST_TIMEOUT, &mut self.task)
            .await
            .expect(CLEANUP_SERVER_DEADLINE)
            .unwrap();
        assert_eq!(
            self.requests.load(std::sync::atomic::Ordering::SeqCst),
            RETRIED_TERMINATION_REQUESTS
        );
    }
}

impl Drop for CleanupTerminationServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve_cleanup_termination(
    listener: UnixListener,
    record: RuntimeRecord,
    error: ErrorBody,
    requests: Arc<std::sync::atomic::AtomicUsize>,
) {
    tokio::time::timeout(
        HOST_REQUEST_TIMEOUT,
        serve_cleanup_terminations(
            listener,
            record,
            error,
            requests,
            SINGLE_TERMINATION_REQUEST,
        ),
    )
    .await
    .expect(CLEANUP_SERVER_DEADLINE);
}

async fn serve_cleanup_terminations(
    listener: UnixListener,
    record: RuntimeRecord,
    error: ErrorBody,
    requests: Arc<std::sync::atomic::AtomicUsize>,
    expected: usize,
) {
    let (stream, _) = listener.accept().await.unwrap();
    drop(listener);
    let mut stream = accept_worker_auth(UnixStream::from(stream), &record).await;
    for _ in 0..expected {
        let request: RequestEnvelope = stream.request().await.unwrap();
        assert_eq!(request.method, method::TERMINATE_RUNTIME_EXACT);
        let payload: crate::worker::FencedTerminateRequest = request.decode_payload().unwrap();
        assert_eq!(payload.terminal_id, record.terminal_id);
        assert_eq!(payload.runtime_epoch, record.runtime_epoch);
        assert_eq!(payload.expected_status, None);
        requests.fetch_add(
            SINGLE_TERMINATION_REQUEST,
            std::sync::atomic::Ordering::SeqCst,
        );
        stream
            .respond(&ResponseEnvelope::failure(&request, error.clone()))
            .await
            .unwrap();
    }
}
