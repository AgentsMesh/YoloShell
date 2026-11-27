use crate::constants::RECEIPT_INVALID;
use anyhow::{Result, ensure};
use yolo_hub_local_startup::{BootstrapDocument, ConnectionDocument};
use yolo_local_profile::LocalProfile;

pub(crate) fn validate(profile: &LocalProfile, bytes: &[u8]) -> Result<ConnectionDocument> {
    let received: BootstrapDocument = yolo_capnp_codec::decode(bytes)?;
    // Reuse the helper's exact schema, persisted identity, home and socket mapping
    // validation. Equality additionally binds the outer schema and reported paths.
    let expected = BootstrapDocument::new(
        &profile.state,
        &profile.host_home,
        received.connection.clone(),
    )?;
    ensure!(received == expected, RECEIPT_INVALID);
    Ok(received.connection)
}
