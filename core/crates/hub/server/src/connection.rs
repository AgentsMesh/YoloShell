use crate::{attachment, constants::*, operation};
use anyhow::{Result, ensure};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Instant;
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_jsonrpc_protocol::{Answer, Call, Fault, PARSE_ERROR, decode};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, Reply, Response};
use yolo_hub_state::HubState;
use yolo_local_unix::LocalUnixStream;

pub(crate) fn same_user(stream: &LocalUnixStream) -> bool {
    stream
        .peer_cred()
        .is_ok_and(|peer| peer.uid() == unsafe { libc::geteuid() })
}

pub(crate) async fn serve(
    stream: LocalUnixStream,
    dispatcher: Arc<dyn Dispatcher>,
    state: Arc<HubState>,
) -> Result<()> {
    ensure!(same_user(&stream), WRONG_LOCAL_USER);
    serve_stream(stream, dispatcher, state).await
}

pub(crate) async fn serve_stream<S: AsyncRead + AsyncWrite + Send + Unpin>(
    stream: S,
    dispatcher: Arc<dyn Dispatcher>,
    state: Arc<HubState>,
) -> Result<()> {
    let mut channel = Framed::new(stream);
    let mut greeted = false;
    loop {
        let Incoming::Message(bytes) = channel
            .receive_frame(Instant::now() + CONTROL_IDLE_TIMEOUT)
            .await?
        else {
            return Ok(());
        };
        let decoded = decode(&bytes);
        if decoded.as_ref().is_ok_and(Call::is_notification) {
            continue;
        }
        let request = match decoded
            .map_err(|_| Fault::new(None, PARSE_ERROR))
            .and_then(Call::parse)
        {
            Ok(request) => request,
            Err(fault) => {
                channel
                    .send(&Answer::failure(fault), Instant::now() + OPERATION_TIMEOUT)
                    .await?;
                continue;
            }
        };
        state.verify()?;
        let fields = request.fields();
        if !greeted {
            ensure!(
                fields.destination == state.hub_id() && matches!(fields.command, Command::Hello {}),
                EXPECTED_HELLO
            );
            greeted = true;
        }
        let result = operation::dispatch(dispatcher.as_ref(), &request).await;
        state.verify()?;
        let reply = match result {
            Ok(Operation::Attached(source)) => {
                yolo_hub_terminal_wire::send_attachment(
                    &mut channel,
                    &request,
                    source.initial(),
                    Instant::now() + OPERATION_TIMEOUT,
                )
                .await?;
                return attachment::serve(channel.into_stream()?, source, state).await;
            }
            Ok(Operation::Reply(reply)) => reply,
            Err(code) => Reply::Error { code },
        };
        respond(&mut channel, &request, reply).await?;
    }
}

async fn respond<S: AsyncWrite + Unpin>(
    channel: &mut Framed<S>,
    request: &Call,
    reply: Reply,
) -> Result<()> {
    Response::new(request, request.fields().destination, reply.clone()).validate_for(request)?;
    let response = Answer::new(request, reply)?;
    channel
        .send(&response, Instant::now() + OPERATION_TIMEOUT)
        .await?;
    Ok(())
}
