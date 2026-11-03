use super::{RelayError, RelayRuntime, TerminalSize};
use crate::client::{ClientError, RpcConnection};
use crate::model::WriterCapability;
use crate::protocol::method;
#[cfg(test)]
use crate::worker::SignalRequest;
use crate::worker::{InputRequest, InputResult, ResizeRequest, RuntimeRef};
use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

#[path = "control/retry.rs"]
mod retry;

pub(crate) struct ControlChannel {
    runtime: RelayRuntime,
    target: RuntimeRef,
    connection: Option<RpcConnection>,
}

#[derive(Clone, Copy)]
enum RetrySafety {
    Idempotent,
    Deduplicated,
    DeliveryMayBeUnknown,
}

impl RetrySafety {
    fn permits_retry(self) -> bool {
        match self {
            Self::Idempotent | Self::Deduplicated => true,
            Self::DeliveryMayBeUnknown => false,
        }
    }
}

impl ControlChannel {
    pub(crate) async fn connect(
        runtime: RelayRuntime,
        target: RuntimeRef,
    ) -> Result<Self, RelayError> {
        let connection = Some(runtime.client().connect().await?);
        Ok(Self {
            runtime,
            target,
            connection,
        })
    }

    pub(crate) async fn input(
        &mut self,
        generation: u64,
        writer_capability: &WriterCapability,
        input_id: Uuid,
        data: &[u8],
    ) -> Result<InputResult, RelayError> {
        let request = InputRequest {
            terminal_id: self.target.terminal_id,
            runtime_epoch: self.target.runtime_epoch,
            generation,
            writer_capability: writer_capability.clone(),
            input_id,
            data: data.to_vec(),
        };
        self.request(method::TERMINAL_INPUT, &request, RetrySafety::Deduplicated)
            .await
    }

    pub(crate) async fn resize(
        &mut self,
        generation: u64,
        writer_capability: &WriterCapability,
        size: TerminalSize,
    ) -> Result<(), RelayError> {
        let request = ResizeRequest {
            terminal_id: self.target.terminal_id,
            runtime_epoch: self.target.runtime_epoch,
            generation,
            writer_capability: writer_capability.clone(),
            cols: size.cols,
            rows: size.rows,
        };
        self.request(method::RESIZE_TERMINAL, &request, RetrySafety::Idempotent)
            .await
    }

    #[cfg(test)]
    pub(crate) async fn signal(
        &mut self,
        generation: u64,
        writer_capability: &WriterCapability,
        signal: impl Into<String>,
    ) -> Result<(), RelayError> {
        let request = SignalRequest {
            terminal_id: self.target.terminal_id,
            runtime_epoch: self.target.runtime_epoch,
            generation,
            writer_capability: writer_capability.clone(),
            signal: signal.into(),
        };
        self.request(
            method::SIGNAL_SESSION,
            &request,
            RetrySafety::DeliveryMayBeUnknown,
        )
        .await
    }

    async fn request<T, R>(
        &mut self,
        method_name: &str,
        payload: &T,
        retry: RetrySafety,
    ) -> Result<R, RelayError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        if matches!(retry, RetrySafety::Idempotent | RetrySafety::Deduplicated) {
            return self.request_replay_safe(method_name, payload).await;
        }
        let result = self
            .connection
            .as_mut()
            .expect("control connection initialized")
            .request(method_name, payload)
            .await;
        let Err(error) = result else {
            return result.map_err(Into::into);
        };
        if !super::host::is_transport_error(&error) {
            return Err(error.into());
        }
        self.connection = None;
        self.runtime.reconnect(self.target.terminal_id).await?;
        self.connection = Some(self.runtime.client().connect().await?);
        if !retry.permits_retry() {
            return Err(error.into());
        }
        self.connection
            .as_mut()
            .expect("control connection reconnected")
            .request(method_name, payload)
            .await
            .map_err(Into::into)
    }
}

pub(crate) fn client_error(error: &RelayError) -> Option<&ClientError> {
    match error {
        RelayError::Client(error) => Some(error),
        _ => None,
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/control_retry_tests.rs"]
mod retry_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/control_tests.rs"]
mod tests;
