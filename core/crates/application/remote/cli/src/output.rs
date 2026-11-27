use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::io::Write;
use yolo_remote_broker::{BrokerResult, BrokerStatus};

const CONNECTION_MISSING: &str = "broker identity has no remote connection";
const GENERATION_INVALID: &str = "broker identity has an invalid connection generation";

#[derive(Serialize)]
struct BrokerIdentityOutput {
    protocol_version: u16,
    instance_id: uuid::Uuid,
    host_id: String,
    config_revision: String,
    connection_id: uuid::Uuid,
    connection_generation: u64,
}

#[derive(Serialize)]
struct BootstrapOutput {
    bootstrap: BrokerIdentityOutput,
}

#[derive(Serialize)]
struct StartOutput {
    start: BrokerIdentityOutput,
}

impl BrokerIdentityOutput {
    fn from_status(status: BrokerStatus) -> Result<Self> {
        let connection_id = status.connection_id.context(CONNECTION_MISSING)?;
        ensure!(
            status.connection_generation > u64::default(),
            GENERATION_INVALID
        );
        Ok(Self {
            protocol_version: status.protocol_version,
            instance_id: status.instance_id,
            host_id: status.host_id,
            config_revision: status.config_revision,
            connection_id,
            connection_generation: status.connection_generation,
        })
    }
}

pub(super) fn print_bootstrap(status: BrokerStatus) -> Result<()> {
    print(&BootstrapOutput {
        bootstrap: BrokerIdentityOutput::from_status(status)?,
    })
}

pub(super) fn print_start(status: BrokerStatus) -> Result<()> {
    print(&StartOutput {
        start: BrokerIdentityOutput::from_status(status)?,
    })
}

pub(super) fn print_result(result: BrokerResult) -> Result<()> {
    print(&result)
}

pub(super) fn print(value: &impl serde::Serialize) -> Result<()> {
    write_output(&mut std::io::stdout().lock(), value)
}

fn write_output(output: &mut impl Write, value: &impl Serialize) -> Result<()> {
    let encoded = yolo_capnp_codec::encode(value)?;
    output.write_all(&encoded)?;
    output.flush()?;
    Ok(())
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/output_tests.rs"]
mod tests;
