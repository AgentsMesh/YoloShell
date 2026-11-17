const INVALID: &str = "native callback Linux executable lineage changed";
const MISSING: &str = "native callback ancestor is absent from the retained observation";
use anyhow::{Context, Result, ensure};
use std::collections::HashSet;
use yolo_agent_contract::ObservedProcess;
use yolo_process::{ProcessIdentity, linux_native};

pub fn normalize(
    peer_pid: u32,
    shell: ProcessIdentity,
    processes: &mut [ObservedProcess],
    maximum_depth: usize,
) -> Result<()> {
    let mut pid = peer_pid;
    let mut seen = HashSet::new();
    while pid != shell.pid {
        ensure!(seen.len() < maximum_depth && seen.insert(pid), INVALID);
        let process = processes
            .iter_mut()
            .find(|process| process.pid == pid)
            .with_context(|| format!("{MISSING}: {pid}"))?;
        let before = linux_native::process_metadata(pid)?;
        ensure!(
            before.start_time == process.start_time && before.parent_pid == process.parent_pid,
            INVALID
        );
        // The generic Linux inventory carries comm; admission requires the actual executable path.
        let executable = linux_native::executable_path(pid)?;
        ensure!(linux_native::process_metadata(pid)? == before, INVALID);
        process.executable = executable.to_str().context(INVALID)?.to_owned();
        pid = process.parent_pid;
    }
    Ok(())
}
