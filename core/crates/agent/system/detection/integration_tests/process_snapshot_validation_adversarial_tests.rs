use super::*;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

const TEST_UNKNOWN_ENVIRONMENT_KEY: &str = "UNREGISTERED_AGENT_HOME";
const TEST_OTHER_PID: u32 = 102;
const TEST_OTHER_PROCESS_GROUP: i32 = 54;

#[test]
fn stable_snapshot_rejects_nul_in_os_derived_values() {
    let nul = super::super::super::constants::ENVIRONMENT_NUL;
    let mut executable = valid_process(TEST_PID);
    executable.executable.push(nul);
    assert_invalid(executable);

    let mut argv = valid_process(TEST_PID);
    argv.argv = vec![format!("argument{nul}suffix")];
    assert_invalid(argv);

    let mut environment_key = valid_process(TEST_PID);
    environment_key.environment = Some(BTreeMap::from([(
        format!("{TEST_ENVIRONMENT_KEY}{nul}"),
        TEST_ENVIRONMENT_VALUE.to_owned(),
    )]));
    assert_invalid(environment_key);

    let mut environment_value = valid_process(TEST_PID);
    environment_value.environment = Some(BTreeMap::from([(
        TEST_ENVIRONMENT_KEY.to_owned(),
        format!("{TEST_ENVIRONMENT_VALUE}{nul}"),
    )]));
    assert_invalid(environment_value);

    let mut cwd = valid_process(TEST_PID);
    cwd.cwd = Some(path_with_nul().into());
    assert_invalid(cwd);

    let mut evidence = valid_process(TEST_PID);
    evidence.evidence_paths = Some(vec![path_with_nul().into()]);
    assert_invalid(evidence);
}

#[test]
fn stable_snapshot_rejects_unknown_environment_keys() {
    let mut process = valid_process(TEST_PID);
    process.environment = Some(BTreeMap::from([(
        TEST_UNKNOWN_ENVIRONMENT_KEY.to_owned(),
        TEST_ENVIRONMENT_VALUE.to_owned(),
    )]));
    assert_invalid(process);
}

#[test]
fn stable_snapshot_rejects_invalid_parent_and_group_values() {
    let mut parent_missing = valid_process(TEST_PID);
    parent_missing.parent_pid = u32::default();
    assert_invalid(parent_missing);

    let mut parent_self = valid_process(TEST_PID);
    parent_self.parent_pid = parent_self.pid;
    assert_invalid(parent_self);

    let mut parent_out_of_range = valid_process(TEST_PID);
    parent_out_of_range.parent_pid = u32::MAX;
    assert_invalid(parent_out_of_range);

    let mut process_group_missing = valid_process(TEST_PID);
    process_group_missing.process_group = i32::default();
    assert_invalid(process_group_missing);

    let mut process_group_negative = valid_process(TEST_PID);
    process_group_negative.process_group = i32::MIN;
    assert_invalid(process_group_negative);

    let mut foreground_group_missing = valid_process(TEST_PID);
    foreground_group_missing.foreground_group = i32::default();
    assert_invalid(foreground_group_missing);

    let mut foreground_group_negative = valid_process(TEST_PID);
    foreground_group_negative.foreground_group = i32::MIN;
    assert_invalid(foreground_group_negative);

    let mut groups_differ = valid_process(TEST_PID);
    groups_differ.foreground_group += 1;
    assert_invalid(groups_differ);

    let first_group = valid_process(TEST_PID);
    let mut other_group = valid_process(TEST_OTHER_PID);
    other_group.process_group = TEST_OTHER_PROCESS_GROUP;
    other_group.foreground_group = TEST_OTHER_PROCESS_GROUP;
    assert!(decode_processes(vec![first_group, other_group], &[]).is_err());
}

fn assert_invalid(process: ObservedProcessWire) {
    assert!(decode_processes(vec![process], &[]).is_err());
}

fn path_with_nul() -> PathBuf {
    let mut bytes = TEST_CWD.as_bytes().to_vec();
    bytes.push(super::super::super::constants::NUL_BYTE);
    PathBuf::from(OsString::from_vec(bytes))
}
