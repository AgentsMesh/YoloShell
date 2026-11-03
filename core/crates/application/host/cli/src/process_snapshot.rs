use anyhow::Result;

pub(super) fn write(
    root_pid: u32,
    root_start_time: u64,
    parent_watch: bool,
    environment_keys: Vec<String>,
) -> Result<()> {
    if parent_watch {
        yolo_runtime::agent::start_process_snapshot_parent_watch()?;
    }
    let root = yolo_runtime::process::ProcessIdentity {
        pid: root_pid,
        start_time: root_start_time,
    };
    yolo_runtime::agent::write_system_process_snapshot(
        root,
        &environment_keys,
        std::io::stdout().lock(),
    )
}
