use crate::{configuration, constants::*};
use anyhow::{Result, ensure};
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_enrollment_store::PreparedEnrollment;
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_remote_provisioning_protocol::{
    REMOTE_BOOTSTRAP_SCHEMA_VERSION, RemoteBootstrapReceipt, RemoteBootstrapReceiptFields,
};

pub(crate) fn create(
    prepared: &PreparedEnrollment,
    ready: &ConfigurationSnapshot,
    build: ServiceBuild,
) -> Result<RemoteBootstrapReceipt> {
    ensure!(prepared.applied_to(ready), CONFIGURATION_UNKNOWN);
    let request = &prepared.request;
    configuration::verify_images(&build, request)?;
    let endpoint = ready
        .listen_address
        .ok_or_else(|| anyhow::anyhow!(LISTENER_UNAVAILABLE))?;
    let receipt = RemoteBootstrapReceipt::new(RemoteBootstrapReceiptFields {
        schema_version: REMOTE_BOOTSTRAP_SCHEMA_VERSION,
        request_id: request.fields().request_id,
        peer: prepared.peer,
        endpoint,
        configuration_revision: ready.state.revision,
        bundle_id: request.fields().bundle.build_id().into(),
        build,
        consumer: request.fields().consumer,
        rights: request.fields().rights.clone(),
        remaining_reexports: request.fields().remaining_reexports,
    })?;
    receipt.validate_for(request)?;
    Ok(receipt)
}
