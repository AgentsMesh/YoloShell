use super::*;

#[test]
fn system_api_reads_native_process_fields_without_commands() {
    let api = SystemApi;
    let pid = std::process::id();
    let metadata = api.metadata(pid).unwrap();
    assert_eq!(metadata.pid, pid);
    assert!(
        api.group_pids(metadata.process_group, MAX_FOREGROUND_PROCESS_IDS)
            .unwrap()
            .contains(&pid)
    );
    assert!(api.executable(pid).unwrap().is_absolute());
    assert!(api.cwd(pid).unwrap().is_absolute());
    assert!(api.evidence_paths(pid).is_ok());
    let environment_keys = [crate::agent::constants::YOLO_TERMINAL_ID_ENV];
    let argv = api
        .argv(
            pid,
            crate::agent::constants::MAX_PROCESS_ARGV_BYTES,
            &environment_keys,
        )
        .unwrap();
    assert!(!argv.values.is_empty());
    assert!(argv.bytes > 0);
    assert!(
        argv.environment
            .keys()
            .all(|key| environment_keys.contains(&key.as_str()))
    );
}

#[test]
fn system_source_rejects_invalid_shell_identity() {
    assert!(descendants(u32::MAX).is_err());
}
