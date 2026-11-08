//! Public archived-history read over the Host that owns the retained journal.
//! This decorator sits inside client-access authorization and delegates every
//! unrelated command to the normal Hub dispatcher.

use std::sync::Arc;
use yolo_client::Client;
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_local_errors::map_error;
use yolo_hub_rpc_protocol::{Command, Method, Reply, Request};
use yolo_hub_terminal_protocol::{
    PublicError, ReadTerminalHistoryRequest, TerminalHistoryRead, TerminalHistoryReadFields,
};

/// Origin-side methods implemented by this decorator.
pub const ORIGIN_METHODS: &[Method] = &[Method::ReadTerminalHistory];

pub struct HistoryDispatcher {
    inner: Arc<dyn Dispatcher>,
    hub: HubId,
    client: Client,
}

impl HistoryDispatcher {
    pub fn new(inner: Arc<dyn Dispatcher>, hub: HubId, client: Client) -> Self {
        Self { inner, hub, client }
    }
}

impl Dispatcher for HistoryDispatcher {
    fn hub_id(&self) -> HubId {
        self.hub
    }

    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            if request.fields().destination == self.hub
                && let Command::ReadTerminalHistory { request } = &request.fields().command
            {
                return Ok(Operation::Reply(read(&self.client, request).await?));
            }
            self.inner.dispatch(request).await
        })
    }
}

async fn read(
    client: &Client,
    request: &ReadTerminalHistoryRequest,
) -> Result<Reply, PublicError> {
    let target = request.fields().target;
    let result = client
        .read_retained_history(
            target.address.terminal_id.as_uuid(),
            target.runtime_epoch.as_uuid(),
            request.max_bytes(),
        )
        .await
        .map_err(map_error)?;
    if result.runtime_epoch != target.runtime_epoch.as_uuid() {
        return Err(PublicError::OutcomeUnknown);
    }
    let history = TerminalHistoryRead::new(TerminalHistoryReadFields {
        target,
        data: result.data_base64,
        from_sequence: result.from_sequence,
        next_sequence: result.next_sequence,
        truncated: result.truncated,
    })?;
    Ok(Reply::TerminalHistory { history })
}
