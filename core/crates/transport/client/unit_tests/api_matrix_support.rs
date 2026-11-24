use crate::protocol::RequestEnvelope;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::net::UnixListener;
use tokio::task::JoinHandle;
use tokio::time::Instant;

pub(super) use super::api_matrix_fixtures::{
    MatrixScenario, test_creation_capability, test_input_id, test_runtime_epoch, test_session_id,
    test_target,
};

const MATRIX_SOCKET_NAME: &str = "client-api-matrix.sock";
const MATRIX_OPERATION_BUDGET: Duration = Duration::from_secs(10);
const MATRIX_DEADLINE_EXCEEDED: &str = "client API matrix deadline exceeded";
const MATRIX_SERVER_FAILED: &str = "client API matrix server task failed";

pub(super) struct MatrixRequests {
    pending: Vec<RequestEnvelope>,
}

impl MatrixRequests {
    pub(super) fn take_unique<T: DeserializeOwned>(&mut self, method: &str) -> T {
        self.take_by_typed_key(method, |_| true)
    }

    pub(super) fn take_by_typed_key<T, P>(&mut self, method: &str, matches: P) -> T
    where
        T: DeserializeOwned,
        P: Fn(&T) -> bool,
    {
        let mut selected = None;
        for (index, request) in self.pending.iter().enumerate() {
            if request.method != method {
                continue;
            }
            let payload = request.decode_payload::<T>().unwrap();
            if matches(&payload) {
                assert!(
                    selected.replace(index).is_none(),
                    "ambiguous {method} request key"
                );
            }
        }
        let index = selected.unwrap_or_else(|| panic!("missing {method} request key"));
        self.pending.remove(index).decode_payload().unwrap()
    }

    pub(super) fn assert_empty(self) {
        let methods = self
            .pending
            .iter()
            .map(|request| request.method.as_str())
            .collect::<Vec<_>>();
        assert!(
            methods.is_empty(),
            "unverified matrix requests: {methods:?}"
        );
    }
}

pub(super) struct MatrixServer {
    _directory: TempDir,
    pub(super) socket: PathBuf,
    deadline: Instant,
    task: Option<JoinHandle<Vec<RequestEnvelope>>>,
}

impl MatrixServer {
    pub(super) async fn start(expected: &[&str]) -> Self {
        Self::start_with_scenario(expected, MatrixScenario::Standard).await
    }

    pub(super) async fn start_with_scenario(expected: &[&str], scenario: MatrixScenario) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join(MATRIX_SOCKET_NAME);
        let listener = UnixListener::bind(&socket).unwrap();
        let deadline = Instant::now() + MATRIX_OPERATION_BUDGET;
        let mut remaining = expected
            .iter()
            .map(|method| method.to_string())
            .collect::<Vec<_>>();
        let task = tokio::spawn(async move {
            let mut requests = Vec::with_capacity(remaining.len());
            while !remaining.is_empty() {
                let (stream, _) = by_deadline(deadline, listener.accept()).await.unwrap();
                let mut stream = crate::rpc_test_support::Peer::new(stream);
                let request: RequestEnvelope =
                    by_deadline(deadline, stream.request()).await.unwrap();
                let index = remaining
                    .iter()
                    .position(|method| method == &request.method)
                    .unwrap_or_else(|| panic!("unexpected matrix method {}", request.method));
                remaining.remove(index);
                let response = super::api_matrix_fixtures::response(&request, scenario);
                by_deadline(deadline, stream.respond(&response))
                    .await
                    .unwrap();
                requests.push(request);
            }
            requests
        });
        Self {
            _directory: directory,
            socket,
            deadline,
            task: Some(task),
        }
    }

    pub(super) async fn rpc<F: Future>(&self, future: F) -> F::Output {
        by_deadline(self.deadline, future).await
    }

    pub(super) async fn finish(mut self) -> MatrixRequests {
        let mut task = self.task.take().expect(MATRIX_SERVER_FAILED);
        let requests = match tokio::time::timeout_at(self.deadline, &mut task).await {
            Ok(result) => result.expect(MATRIX_SERVER_FAILED),
            Err(_) => {
                task.abort();
                let _ = task.await;
                panic!("{MATRIX_DEADLINE_EXCEEDED}");
            }
        };
        MatrixRequests { pending: requests }
    }
}

impl Drop for MatrixServer {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

async fn by_deadline<F: Future>(deadline: Instant, future: F) -> F::Output {
    tokio::time::timeout_at(deadline, future)
        .await
        .expect(MATRIX_DEADLINE_EXCEEDED)
}
