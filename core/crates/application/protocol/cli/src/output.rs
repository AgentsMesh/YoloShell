use crate::arguments::{Command, ProtocolCommand};
use anyhow::Result;
use std::io::Write;

const JSON_LINE_END: &[u8] = b"\n";

pub(crate) fn write(command: Command, output: &mut impl Write) -> Result<()> {
    match command {
        Command::Protocol {
            command: ProtocolCommand::Describe { json: _ },
        } => {
            serde_json::to_writer_pretty(
                &mut *output,
                &yolo_hub_rpc_descriptor::ProtocolDescriptor::current(),
            )?;
        }
    }
    output.write_all(JSON_LINE_END)?;
    Ok(())
}

pub(crate) fn write_command(command: ProtocolCommand, output: &mut impl Write) -> Result<()> {
    write(Command::Protocol { command }, output)
}
