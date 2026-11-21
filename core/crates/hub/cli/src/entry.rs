use crate::{bootstrap, constants::PEER_SOCKET_MISSING, options::Options, peer_stdio, remote, serve};
use anyhow::Context;
use clap::Parser;
use std::ffi::OsString;

/// Runs the Hub process role. Arguments include the role's display name as argv[0].
pub fn run_from(argv: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let _ = tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(std::io::stderr)
                .try_init();
            run(Options::try_parse_from(argv)?).await
        })
}

async fn run(options: Options) -> anyhow::Result<()> {
    if options.peer_stdio {
        return peer_stdio::run(options.peer_socket.context(PEER_SOCKET_MISSING)?).await;
    }
    if options.install_bundle
        || options.inspect_install
        || options.remote_bootstrap
        || options.recover_remote_bootstrap
    {
        return remote::run(options).await;
    }
    if options.bootstrap {
        return bootstrap::run(options).await;
    }
    let receiver = options
        .ready_stdio
        .then(yolo_startup_guard::Receiver::from_stdin)
        .transpose()?;
    let stop = async move {
        match receiver {
            Some(receiver) => receiver.cancellation().await,
            None => std::future::pending().await,
        }
    };
    tokio::select! {
        result = serve::run(options) => result,
        _ = stop => Err(anyhow::anyhow!(yolo_startup_guard::CANCELLED)),
    }
}
