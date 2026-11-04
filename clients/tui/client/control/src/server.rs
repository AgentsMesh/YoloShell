use crate::{constants::*, framing, peer};
use anyhow::{Context, Result};
use tokio::{
    net::UnixListener,
    sync::oneshot,
    time::{Instant, timeout_at},
};
use uuid::Uuid;
use yolo_local_unix::LocalUnixStream;
use yolo_tui_client_control_protocol::{ControlCommand, ControlRequest, ControlResponse};

pub(crate) async fn serve(
    listener: UnixListener,
    token: Uuid,
    requested: oneshot::Sender<Instant>,
    finished: oneshot::Receiver<()>,
) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await.context(CONTROL_OWNER_FAILED)?;
        let deadline = Instant::now() + CONTROL_REQUEST_TIMEOUT;
        let mut stream = LocalUnixStream::from(stream);
        if peer::verify(&stream).is_err() {
            continue;
        }
        let request = framing::receive::<_, ControlRequest>(&mut stream, deadline).await;
        let accepted = request
            .as_ref()
            .is_ok_and(|request| request.authorizes(token));
        if request
            .as_ref()
            .is_ok_and(|request| request.command == ControlCommand::Status)
        {
            let _ = framing::send(&mut stream, &ControlResponse::status(accepted), deadline).await;
            continue;
        }
        if !accepted {
            let _ = framing::send(&mut stream, &ControlResponse::rejected(), deadline).await;
            continue;
        }
        requested
            .send(deadline)
            .map_err(|_| anyhow::anyhow!(CONTROL_FINISH_FAILED))?;
        timeout_at(deadline, finished)
            .await
            .context(CONTROL_DEADLINE)?
            .context(CONTROL_FINISH_FAILED)?;
        return framing::send(&mut stream, &ControlResponse::accepted(), deadline).await;
    }
}
