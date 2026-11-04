use serde::Serialize;
use std::io::{IsTerminal, Write};
use crate::ctl_args::RuntimeWriterAuthorization;
use yolo_runtime::model::CreateTerminalResult;

const AUTHORIZATION_TTY_ERROR: &str =
    "refusing to print writer authorization to an interactive terminal";
const CREATION_AUTHORIZATION_TTY_ERROR: &str =
    "refusing to print creation authorization to an interactive terminal";

#[derive(Serialize)]
struct SuccessResponse {
    ok: bool,
}

#[derive(Serialize)]
struct PublicCreationResult {
    terminal_id: uuid::Uuid,
}

#[derive(Serialize)]
struct PublicWriterGrant<'a> {
    target: RuntimeRef,
    lease: &'a yolo_runtime::model::WriterLease,
}

fn print_native<T: Serialize>(value: &T) -> Result<()> {
    write_output(&yolo_capnp_codec::encode(value)?)
}

fn write_output(bytes: &[u8]) -> Result<()> {
    let mut output = std::io::stdout().lock();
    output.write_all(bytes)?;
    output.flush()?;
    Ok(())
}

fn print_native_frame<T: Serialize>(value: &T) -> Result<()> {
    let bytes = yolo_capnp_codec::encode(value)?;
    let mut frame = (bytes.len() as u32).to_be_bytes().to_vec();
    frame.extend(bytes);
    write_output(&frame)
}

fn print_ok() -> Result<()> {
    print_native(&SuccessResponse { ok: true })
}

fn print_writer_grant(target: RuntimeRef, grant: &WriterLeaseGrant, export: bool) -> Result<()> {
    let encoded = encode_writer_grant(target, grant, export, std::io::stdout().is_terminal())?;
    write_output(&encoded)
}

fn print_creation_result(result: &CreateTerminalResult, export: bool) -> Result<()> {
    let encoded = encode_creation_result(result, export, std::io::stdout().is_terminal())?;
    write_output(&encoded)
}

pub(super) fn encode_writer_grant(
    target: RuntimeRef,
    grant: &WriterLeaseGrant,
    export: bool,
    interactive_terminal: bool,
) -> Result<Vec<u8>> {
    anyhow::ensure!(!(export && interactive_terminal), AUTHORIZATION_TTY_ERROR);
    if export {
        Ok(yolo_capnp_codec::encode(&RuntimeWriterAuthorization::new(
            target,
            grant.clone(),
        ))?)
    } else {
        Ok(yolo_capnp_codec::encode(&PublicWriterGrant {
            target,
            lease: &grant.lease,
        })?)
    }
}

pub(super) fn encode_creation_result(
    result: &CreateTerminalResult,
    export: bool,
    interactive_terminal: bool,
) -> Result<Vec<u8>> {
    validate_creation_export(export, interactive_terminal)?;
    if export {
        Ok(yolo_capnp_codec::encode(result)?)
    } else {
        Ok(yolo_capnp_codec::encode(&PublicCreationResult {
            terminal_id: result.terminal_id,
        })?)
    }
}

pub(super) fn validate_creation_export(export: bool, interactive_terminal: bool) -> Result<()> {
    anyhow::ensure!(
        !(export && interactive_terminal),
        CREATION_AUTHORIZATION_TTY_ERROR
    );
    Ok(())
}
