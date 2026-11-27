use super::CtlOptions;
use anyhow::Result;
use yolo_client::Client;
use yolo_runtime::paths::Paths;

pub(super) async fn open(options: &CtlOptions) -> Result<Client> {
    if options.existing_socket {
        let socket = options
            .socket
            .as_ref()
            .expect("clap requires existing socket path");
        yolo_remote_broker::validate_broker_socket(socket)?;
        return Ok(yolo_remote_broker::existing_runtime_client(socket));
    }
    let home = options.home.as_ref().expect("clap requires local home");
    let paths = Paths::new(home);
    let permits_start = !matches!(options.command, super::CtlCommand::ShutdownIfIdle);
    let namespace = if !options.existing_host && permits_start {
        Some(crate::host_namespace::HostNamespace::retain(&paths)?)
    } else {
        None
    };
    let socket = match (options.socket.clone(), &namespace) {
        (Some(socket), _) => socket,
        (None, Some(namespace)) => namespace.socket()?,
        (None, None) => paths.host_socket()?,
    };
    if let Some(namespace) = &namespace {
        super::super::bridge::ensure_host_running(&paths, &socket).await?;
        namespace.verify()?;
    }
    Ok(Client::managed_local(paths.home(), socket)?)
}
