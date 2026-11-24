use super::super::audit::{Failure, Shared};
use crate::hub::support::deadline;
use std::{path::PathBuf, sync::Arc};
use tokio::{io::AsyncReadExt, net::UnixStream};
use yolo_hub_jsonrpc_protocol::Answer;
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, Response};
#[path = "tui_creation_recovery_proxy_wire.rs"]
mod wire;

pub async fn run(
    mut client: UnixStream,
    origin: PathBuf,
    workspace: PathBuf,
    shared: Arc<Shared>,
) -> Result<(), Failure> {
    let origin = tokio::time::timeout_at(deadline(), UnixStream::connect(origin))
        .await
        .map_err(|_| Failure::Connect)?
        .map_err(|_| Failure::Connect)?;
    let mut origin = Framed::new(origin);
    loop {
        let Some(request) = wire::request(&mut client).await? else {
            return Ok(());
        };
        let held = shared.audit.lock().unwrap().observe(&request, &workspace)?;
        origin
            .send(&request, deadline())
            .await
            .map_err(|_| Failure::WriteRequest)?;
        let Incoming::Message(response) = origin
            .receive::<Answer>(deadline())
            .await
            .map_err(|_| Failure::ReadReply)?
        else {
            return Err(Failure::ReadReply);
        };
        if matches!(request.fields().command, Command::Attach { .. }) && response.result.is_some() {
            if response.jsonrpc != yolo_hub_jsonrpc_protocol::JSONRPC_VERSION
                || response.id.as_ref() != Some(&request.id) || response.error.is_some() {
                return Err(Failure::InvalidReply);
            }
            wire::response(&mut client, &response).await?;
            let mut origin = origin.into_stream().map_err(|_| Failure::InvalidReply)?;
            let _ = tokio::io::copy_bidirectional(&mut client, &mut origin).await;
            return Ok(());
        }
        let reply = response
            .clone()
            .reply(&request)
            .map_err(|_| Failure::InvalidReply)?;
        shared.audit.lock().unwrap().reply(
            &request,
            &Response::new(&request, request.fields().destination, reply.clone()),
            held,
        )?;
        shared.changed.notify_one();
        if held {
            // Origin really created this runtime. The frontend never receives
            // its Created bytes; only closing this exact connection releases it.
            match tokio::time::timeout_at(deadline(), client.read_u8()).await {
                Ok(Err(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                    shared.audit.lock().unwrap().held_closed = true;
                    shared.changed.notify_one();
                    return Ok(());
                }
                _ => return Err(Failure::HeldRequest),
            }
        }
        if wire::response(&mut client, &response).await.is_err() {
            // A closing view may abandon a normal in-flight read response.
            return Ok(());
        }
    }
}
