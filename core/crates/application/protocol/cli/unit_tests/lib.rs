use crate::arguments::{Arguments, Command, ProtocolCommand};
use clap::Parser;
use std::ffi::OsString;

#[path = "../test_support/constants.rs"]
mod constants;
mod json_flag;
use constants::*;

struct FailingOutput;
impl std::io::Write for FailingOutput {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn arguments(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

#[test]
fn public_entrypoint_emits_a_description_without_starting_a_hub() {
    assert!(crate::run_from(arguments(&[PROGRAM, crate::PROTOCOL_COMMAND, DESCRIBE])).is_ok());
}

#[test]
fn protocol_description_is_json_and_arguments_are_strict() {
    let parsed = Arguments::try_parse_from([PROGRAM, crate::PROTOCOL_COMMAND, DESCRIBE]).unwrap();
    let mut bytes = Vec::new();
    crate::output::write(parsed.command, &mut bytes).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(wire["protocol"], yolo_hub_rpc_descriptor::PROTOCOL_NAME);
    assert!(wire["methods"].as_array().unwrap().len() > 1);
    for values in [
        vec![PROGRAM, "plugins"],
        vec![PROGRAM, crate::PROTOCOL_COMMAND, DESCRIBE, "extra"],
    ] {
        assert!(crate::run_from(arguments(&values)).is_err());
    }
}

#[test]
fn protocol_writes_report_failures() {
    let command = Command::Protocol {
        command: ProtocolCommand::Describe { json: false },
    };
    assert!(crate::output::write(command, &mut FailingOutput).is_err());
}

#[test]
fn dispatch_only_intercepts_protocol() {
    assert!(crate::selects(&arguments(&[
        PROGRAM,
        crate::PROTOCOL_COMMAND
    ])));
    for values in [
        vec![],
        vec![PROGRAM],
        vec![PROGRAM, "plugins"],
        vec![PROGRAM, HUB_ARGUMENT, crate::PROTOCOL_COMMAND],
    ] {
        assert!(!crate::selects(&arguments(&values)));
    }
}
