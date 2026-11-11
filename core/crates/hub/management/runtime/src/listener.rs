use anyhow::Result;
use std::net::{SocketAddr, TcpListener};
use yolo_hub_listener_binding::{ListenerBindings, persistent};
use yolo_hub_management_protocol::ConfigurationState;

pub(crate) fn restore(
    bindings: &mut ListenerBindings,
    configuration: &ConfigurationState,
) -> Result<Option<TcpListener>> {
    let requested = configuration.configuration.listen;
    let saved = bindings.address(configuration)?;
    let listener = prepare(saved.or(requested))?;
    if persistent(requested) && saved.is_none() {
        bindings.establish(
            configuration,
            address(&listener)?
                .ok_or_else(|| anyhow::anyhow!(crate::constants::LISTENER_MISSING))?,
        )?;
    }
    verify(bindings, configuration, &listener)?;
    Ok(listener)
}

pub(crate) fn verify(
    bindings: &ListenerBindings,
    configuration: &ConfigurationState,
    listener: &Option<TcpListener>,
) -> Result<()> {
    bindings.verify(configuration)?;
    if persistent(configuration.configuration.listen) {
        anyhow::ensure!(
            bindings.address(configuration)? == address(listener)?,
            crate::constants::LISTENER_MISSING
        );
    }
    Ok(())
}

pub(crate) fn prepare(address: Option<SocketAddr>) -> Result<Option<TcpListener>> {
    address
        .map(|address| {
            let listener = TcpListener::bind(address)?;
            listener.set_nonblocking(true)?;
            Ok(listener)
        })
        .transpose()
}

pub(crate) fn activate(listener: &Option<TcpListener>) -> Result<Option<tokio::net::TcpListener>> {
    listener
        .as_ref()
        .map(|listener| Ok(tokio::net::TcpListener::from_std(listener.try_clone()?)?))
        .transpose()
}

pub(crate) fn address(listener: &Option<TcpListener>) -> Result<Option<SocketAddr>> {
    Ok(listener.as_ref().map(TcpListener::local_addr).transpose()?)
}
