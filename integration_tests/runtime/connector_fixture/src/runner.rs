use crate::{Arguments, FixtureError, constants::*, lines};
use tokio::{
    io::BufReader,
    time::{Instant, timeout_at},
};
use yolo_hub_connector_client::{Client, ClientError};
use yolo_hub_connector_protocol::{Command, Reply, Request, Response};
use yolo_hub_identity::HubId;
use yolo_hub_terminal_io::Descriptor;

pub async fn run(arguments: Arguments) -> Result<(), FixtureError> {
    let (input, mut output) = Descriptor::pair(libc::STDIN_FILENO, libc::STDOUT_FILENO)
        .map_err(|_| FixtureError::Input)?;
    let mut client = Client::connect_local(
        &arguments.socket,
        arguments.hub,
        Instant::now() + OPERATION_TIMEOUT,
    )
    .await?;
    let mut input = BufReader::with_capacity(INPUT_BUFFER_BYTES, input);
    loop {
        let next = timeout_at(Instant::now() + INPUT_IDLE_TIMEOUT, lines::next(&mut input))
            .await
            .map_err(|_| FixtureError::Deadline)??;
        let Some(line) = next else {
            break;
        };
        let request = decode(&line, arguments.hub)?;
        let deadline = Instant::now() + OPERATION_TIMEOUT;
        let reply = match client.call(request.clone(), deadline).await {
            Ok(reply) => reply,
            Err(ClientError::Remote(code)) => Reply::Error { code },
            Err(error) => return Err(error.into()),
        };
        let response = Response::new(&request, reply);
        response
            .validate_for(&request)
            .map_err(|_| FixtureError::InvalidRequest)?;
        timeout_at(deadline, lines::write(&mut output, &response))
            .await
            .map_err(|_| FixtureError::Deadline)??;
    }
    client.close(Instant::now() + OPERATION_TIMEOUT).await?;
    Ok(())
}

pub(crate) fn decode(line: &[u8], hub: HubId) -> Result<Request, FixtureError> {
    if line.len() > MAX_LINE_BYTES {
        return Err(FixtureError::TooLarge);
    }
    let request: Request =
        serde_json::from_slice(line).map_err(|_| FixtureError::InvalidRequest)?;
    request
        .validate()
        .map_err(|_| FixtureError::InvalidRequest)?;
    if request.hub != hub
        || !matches!(
            request.command,
            Command::Hello {} | Command::Activate { .. } | Command::Publish { .. }
        )
    {
        return Err(FixtureError::InvalidRequest);
    }
    Ok(request)
}
