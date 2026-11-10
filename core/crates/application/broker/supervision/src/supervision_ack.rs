use anyhow::{Context, Result, ensure};
use std::time::Instant;
use uuid::Uuid;
use yolo_local_unix::BlockingLocalUnixStream;
use yolo_remote_broker::BrokerAdoptionAcknowledgement;

use crate::supervision_frame::{self as frame, Adoption, AdoptionPhase};
const ACK_CONTEXT: &str = "exchange broker supervision adoption acknowledgement";
const ACK_MARKER_INVALID: &str = "broker supervision adoption acknowledgement is invalid";
const ACK_INSTANCE_MISMATCH: &str = "broker supervision adoption acknowledgement instance mismatch";
const ACK_NONCE_MISMATCH: &str = "broker supervision adoption acknowledgement nonce mismatch";

pub(super) struct PreparedAdoption {
    pub(super) nonce: Uuid,
    pub(super) instance_id: Uuid,
}

pub(super) fn writer(
    stream: &BlockingLocalUnixStream,
    adoption_nonce: Uuid,
    deadline: Instant,
    settled: std::sync::mpsc::Sender<bool>,
) -> Result<BrokerAdoptionAcknowledgement> {
    let mut stream = stream.try_clone().context(ACK_CONTEXT)?;
    Ok(Box::new(move |instance_id| {
        let accepted = broker_exchange(&mut stream, adoption_nonce, instance_id, deadline).is_ok();
        let _ = settled.send(accepted);
        accepted
    }))
}

pub(super) fn read_prepared(
    stream: &mut BlockingLocalUnixStream,
    nonce: Uuid,
    instance_id: Uuid,
    deadline: Instant,
) -> Result<PreparedAdoption> {
    set_deadline(stream, deadline)?;
    read_identity_frame(stream, AdoptionPhase::Prepared, nonce, instance_id)?;
    Ok(PreparedAdoption { nonce, instance_id })
}

pub(super) fn commit(
    stream: &mut BlockingLocalUnixStream,
    prepared: PreparedAdoption,
    deadline: Instant,
) -> Result<()> {
    set_deadline(stream, deadline)?;
    write_identity_frame(
        stream,
        AdoptionPhase::Commit,
        prepared.nonce,
        prepared.instance_id,
    )?;
    read_identity_frame(
        stream,
        AdoptionPhase::Committed,
        prepared.nonce,
        prepared.instance_id,
    )
}

fn broker_exchange(
    stream: &mut BlockingLocalUnixStream,
    nonce: Uuid,
    instance_id: Uuid,
    deadline: Instant,
) -> Result<()> {
    set_deadline(stream, deadline)?;
    write_identity_frame(stream, AdoptionPhase::Prepared, nonce, instance_id)?;
    read_identity_frame(stream, AdoptionPhase::Commit, nonce, instance_id)?;
    write_identity_frame(stream, AdoptionPhase::Committed, nonce, instance_id)
}

fn read_identity_frame(
    stream: &mut BlockingLocalUnixStream,
    phase: AdoptionPhase,
    nonce: Uuid,
    instance_id: Uuid,
) -> Result<()> {
    let observed: Adoption = frame::read(stream).context(ACK_CONTEXT)?;
    ensure!(observed.phase == phase, ACK_MARKER_INVALID);
    ensure!(observed.nonce == nonce, ACK_NONCE_MISMATCH);
    ensure!(observed.instance_id == instance_id, ACK_INSTANCE_MISMATCH);
    Ok(())
}

fn write_identity_frame(
    stream: &mut BlockingLocalUnixStream,
    phase: AdoptionPhase,
    nonce: Uuid,
    instance_id: Uuid,
) -> Result<()> {
    frame::write(
        stream,
        &Adoption {
            phase,
            nonce,
            instance_id,
        },
    )
    .context(ACK_CONTEXT)
}

pub(super) fn set_deadline(stream: &BlockingLocalUnixStream, deadline: Instant) -> Result<()> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .context(ACK_CONTEXT)?;
    stream
        .set_read_timeout(Some(remaining))
        .context(ACK_CONTEXT)?;
    stream
        .set_write_timeout(Some(remaining))
        .context(ACK_CONTEXT)
}

#[cfg(test)]
#[cfg(feature = "yolo-integration-tests")]
#[path = "../integration_tests/supervision_ack_tests.rs"]
mod tests;
