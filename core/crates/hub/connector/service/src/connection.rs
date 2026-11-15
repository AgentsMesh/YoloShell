use crate::{constants::*, dispatch, role::Role};
use anyhow::{Result, ensure};
use std::sync::Arc;
use tokio::time::{Instant, timeout};
use yolo_hub_connector_jsonrpc::{Answer, Call};
use yolo_hub_connector_protocol::{Command, ConnectorError, Reply, Response};
use yolo_hub_jsonrpc_protocol::{Fault, PARSE_ERROR, decode};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_state::HubState;
use yolo_local_unix::LocalUnixStream;

pub(crate) fn same_user(stream: &LocalUnixStream) -> bool {
    stream
        .peer_cred()
        .is_ok_and(|peer| peer.uid() == unsafe { libc::geteuid() })
}
pub(crate) async fn serve(
    stream: LocalUnixStream,
    state: Arc<HubState>,
    backend: LocalBackend,
) -> Result<()> {
    ensure!(same_user(&stream), WRONG_OWNER);
    let mut framed = Framed::new(stream);
    let mut role = Role::default();
    let result = run(&mut framed, &state, &backend, &mut role).await;
    if let Some(activation) = role.activation {
        let disconnected = backend.disconnect_connector(activation).await;
        if result.is_ok() {
            disconnected?;
        }
    }
    result
}
async fn run(
    framed: &mut Framed<LocalUnixStream>,
    state: &HubState,
    backend: &LocalBackend,
    role: &mut Role,
) -> Result<()> {
    loop {
        let Incoming::Message(bytes) = framed
            .receive_frame(Instant::now() + IDLE_TIMEOUT)
            .await
            .map_err(|_| anyhow::anyhow!(INVALID_FRAME))?
        else {
            return Ok(());
        };
        let decoded = decode(&bytes);
        if decoded
            .as_ref()
            .is_ok_and(yolo_hub_jsonrpc_protocol::Call::is_notification)
        {
            continue;
        }
        let request = match decoded
            .map_err(|_| Fault::new(None, PARSE_ERROR))
            .and_then(Call::parse)
        {
            Ok(call) => call,
            Err(fault) => {
                framed
                    .send(
                        &yolo_hub_jsonrpc_protocol::Answer::failure(fault),
                        Instant::now() + OPERATION_TIMEOUT,
                    )
                    .await?;
                continue;
            }
        };
        state.verify()?;
        ensure!(request.hub == state.hub_id(), WRONG_HUB);
        if !role.greeted {
            ensure!(matches!(request.command, Command::Hello {}), EXPECTED_HELLO);
            role.greeted = true;
        }
        let reply = match timeout(
            OPERATION_TIMEOUT,
            dispatch::dispatch(backend, role, &request),
        )
        .await
        {
            Ok(Ok(reply)) => reply,
            Ok(Err(code)) => Reply::Error { code },
            Err(_) => Reply::Error {
                code: if request.command.mutates() {
                    ConnectorError::OutcomeUnknown
                } else {
                    ConnectorError::Unavailable
                },
            },
        };
        state.verify()?;
        let response = Response::new(&request, reply);
        response.validate_for(&request)?;
        framed
            .send(
                &Answer::new(&request, response.reply)?,
                Instant::now() + OPERATION_TIMEOUT,
            )
            .await?;
    }
}
