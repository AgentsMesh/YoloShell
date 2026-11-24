use super::*;
use crate::protocol::{ErrorBody, ResponseEnvelope, to_value};
use serde::Serialize;
use tokio::{net::UnixListener, task::JoinHandle};

pub(super) struct ReplyServer {
    _directory: tempfile::TempDir,
    pub client: Client,
    task: Option<JoinHandle<RequestEnvelope>>,
}

impl ReplyServer {
    pub async fn start<T: Serialize>(method: &'static str, payload: &T) -> Self {
        Self::serve(method, Ok(to_value(payload).unwrap())).await
    }

    pub async fn failure(method: &'static str, error: ErrorBody) -> Self {
        Self::serve(method, Err(error)).await
    }

    async fn serve(
        method: &'static str,
        response: Result<yolo_capnp_value::Value, ErrorBody>,
    ) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join(SOCKET_NAME);
        let listener = UnixListener::bind(&socket).unwrap();
        let task = tokio::spawn(async move {
            tokio::time::timeout(OPERATION_TIMEOUT, async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut stream = crate::rpc_test_support::Peer::new(stream);
                let request: RequestEnvelope = stream.request().await.unwrap();
                assert_eq!(request.method, method);
                let response = match response {
                    Ok(payload) => {
                        let mut response = ResponseEnvelope::empty(&request);
                        response.payload = payload;
                        response
                    }
                    Err(error) => ResponseEnvelope::failure(&request, error),
                };
                stream.respond(&response).await.unwrap();
                request
            })
            .await
            .unwrap()
        });
        Self {
            _directory: directory,
            client: Client::new(socket).with_operation_timeout(OPERATION_TIMEOUT),
            task: Some(task),
        }
    }

    pub async fn finish(mut self) -> RequestEnvelope {
        tokio::time::timeout(OPERATION_TIMEOUT, self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap()
    }
}

impl Drop for ReplyServer {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
