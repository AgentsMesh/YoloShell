const RUNTIME_RELEASE_DEVICE: &str = "/dev/null";
const RUNTIME_RELEASE_CAPABILITY: &str = "runtime-output-release-barrier";

fn runtime_release_authority(
    record: &RuntimeRecord,
) -> crate::process::PtyMasterAliasReleaseAuthority {
    let mut proof = crate::process::PtySessionProof::capture(
        RUNTIME_RELEASE_DEVICE.into(),
        RUNTIME_RELEASE_CAPABILITY.to_owned(),
    )
    .unwrap();
    proof
        .bind_runtime(record.terminal_id, record.runtime_epoch)
        .unwrap();
    proof
        .bind_master_alias_release(crate::process::ProcessIdentity {
            pid: record.child_pid,
            start_time: record.child_start_time,
        })
        .unwrap()
}
