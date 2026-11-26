use super::{ServiceRole, ServiceUnit, reload_managers, rollback_image};
use crate::supervision::service_config::ImageChange;
use anyhow::{Result, ensure};

pub(crate) async fn rollback_images(
    changes: &[ImageChange],
    units: &[ServiceUnit],
    restart_services: bool,
) -> Result<()> {
    let hubs = units
        .iter()
        .filter(|unit| unit.role() == ServiceRole::Hub)
        .collect::<Vec<_>>();
    let hosts = units
        .iter()
        .filter(|unit| unit.role() == ServiceRole::Host)
        .collect::<Vec<_>>();
    ensure!(
        hubs.len() == 1 && hosts.len() == 1,
        "managed profile service set changed during recovery"
    );
    let hub = hubs[0];
    let host = hosts[0];
    let mut failure = None;
    if restart_services {
        if let Err(error) = host.stop().await {
            failure.get_or_insert(error);
        }
        if let Err(error) = hub.stop().await {
            failure.get_or_insert(error);
        }
    }
    for change in changes.iter().rev() {
        if let Err(error) = rollback_image(change) {
            failure.get_or_insert(error);
        }
    }
    if let Err(error) = reload_managers(units).await {
        failure.get_or_insert(error);
    }
    if restart_services {
        if let Err(error) = host.start().await {
            failure.get_or_insert(error);
        }
        if let Err(error) = hub.start().await {
            failure.get_or_insert(error);
        }
    }
    failure.map_or(Ok(()), Err)
}
