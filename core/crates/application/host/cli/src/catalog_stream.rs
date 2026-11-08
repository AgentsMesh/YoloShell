use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncWrite};
use yolo_client::ClientError;
use yolo_runtime::cli::constants as cli;
use yolo_runtime::paths::Paths;

#[path = "catalog_stream/catalog_stream_connection.rs"]
mod catalog_stream_connection;
#[path = "catalog_stream/catalog_stream_framing.rs"]
mod catalog_stream_framing;
use catalog_stream_connection::CatalogConnection;
use catalog_stream_framing::{read_catalog_frame, write_catalog_frame};

#[derive(Debug, Args)]
pub(super) struct CatalogStreamOptions {
    #[arg(long, required_unless_present = "existing_socket")]
    home: Option<PathBuf>,
    #[arg(long)]
    socket: Option<PathBuf>,
    #[arg(long, requires = "socket", conflicts_with = "home")]
    existing_socket: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CatalogRequest {
    id: uuid::Uuid,
    command: CatalogCommand,
    deadline_ms: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum CatalogCommand {
    List,
}

#[derive(Serialize)]
struct CatalogResponse<T> {
    id: uuid::Uuid,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    sessions: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<CatalogFailure>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum CatalogFailureKind {
    Transport,
    Remote,
    Configuration,
}

#[derive(Serialize)]
struct CatalogFailure {
    kind: CatalogFailureKind,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retryable: Option<bool>,
}

pub(super) async fn run(options: CatalogStreamOptions) -> Result<()> {
    let (connection, _namespace) = if options.existing_socket {
        (
            CatalogConnection::existing(yolo_remote_broker::existing_runtime_client(
                options.socket.expect("clap requires socket"),
            )),
            None,
        )
    } else {
        let home = options.home.expect("clap requires home");
        let paths = Paths::new(home);
        let namespace = crate::host_namespace::HostNamespace::retain(&paths)?;
        let socket = options
            .socket
            .map(Ok)
            .unwrap_or_else(|| namespace.socket())?;
        (CatalogConnection::managed(paths, socket)?, Some(namespace))
    };
    serve(connection, tokio::io::stdin(), tokio::io::stdout()).await
}

async fn serve<R, W>(mut connection: CatalogConnection, mut input: R, mut output: W) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    loop {
        let request: CatalogRequest = match read_catalog_frame(&mut input).await {
            Ok(request) => request,
            Err(catalog_stream_framing::FrameError::EndOfStream) => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let deadline = catalog_deadline(request.deadline_ms);
        let result = match (request.command, deadline) {
            (CatalogCommand::List, Ok(deadline)) => {
                connection.list_sessions_with_timeout(deadline).await
            }
            (_, Err(error)) => Err(error),
        };
        let response = match result {
            Ok(sessions) => CatalogResponse {
                id: request.id,
                ok: true,
                sessions: Some(sessions),
                error: None,
            },
            Err(error) => CatalogResponse {
                id: request.id,
                ok: false,
                sessions: None,
                error: Some(classify_failure(error)),
            },
        };
        write_catalog_frame(&mut output, &response)
            .await
            .context("write catalog stream response")?;
    }
}

fn catalog_deadline(milliseconds: u64) -> Result<std::time::Duration> {
    anyhow::ensure!(
        (cli::CATALOG_MINIMUM_DEADLINE_MILLISECONDS..=cli::CATALOG_MAXIMUM_DEADLINE_MILLISECONDS)
            .contains(&milliseconds),
        "catalog request deadline is outside policy"
    );
    Ok(std::time::Duration::from_millis(milliseconds))
}

fn classify_failure(error: anyhow::Error) -> CatalogFailure {
    if let Some(ClientError::Remote {
        code,
        message,
        retryable,
        ..
    }) = error.downcast_ref::<ClientError>()
    {
        return CatalogFailure {
            kind: CatalogFailureKind::Remote,
            message: message.clone(),
            code: Some(code.clone()),
            retryable: Some(*retryable),
        };
    }
    let kind = if catalog_stream_connection::is_transport_error(&error) {
        CatalogFailureKind::Transport
    } else {
        CatalogFailureKind::Configuration
    };
    CatalogFailure {
        kind,
        message: error.to_string(),
        code: None,
        retryable: None,
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/catalog_stream/catalog_stream_entry_tests.rs"]
mod entry_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/catalog_stream/catalog_stream_failure_tests.rs"]
mod failure_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/catalog_stream/catalog_stream_tests.rs"]
mod tests;
