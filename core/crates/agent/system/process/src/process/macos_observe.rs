use super::super::ObservedProcess;
use super::budget::ScanBudget;
use super::macos::ProcessApi;
use crate::process::macos_native::ProcessMetadata;
use anyhow::Result;

pub(super) fn observe_process(
    api: &impl ProcessApi,
    info: ProcessMetadata,
    budget: &mut ScanBudget,
    environment_keys: &[&str],
) -> Result<Option<ObservedProcess>> {
    let Ok(executable) = api.executable(info.pid) else {
        return Ok(None);
    };
    budget.record_path(&executable)?;
    let cwd = api.cwd(info.pid).ok();
    if let Some(path) = cwd.as_deref() {
        budget.record_path(path)?;
    }
    let Ok(argv) = api.argv(info.pid, budget.argv_limit(), environment_keys) else {
        return Ok(None);
    };
    budget.record_argv(argv.bytes)?;
    let evidence_paths = match api.evidence_paths(info.pid) {
        Ok(paths) => {
            for path in &paths {
                budget.record_path(path)?;
            }
            Some(paths)
        }
        Err(_) => None,
    };
    let Ok(stable_executable) = api.executable(info.pid) else {
        return Ok(None);
    };
    budget.record_path(&stable_executable)?;
    let stable_cwd = cwd.as_ref().and_then(|_| api.cwd(info.pid).ok());
    if stable_executable != executable || stable_cwd.as_ref() != cwd.as_ref() {
        return Ok(None);
    }
    Ok(Some(ObservedProcess {
        pid: info.pid,
        start_time: info.start_time,
        parent_pid: info.parent_pid,
        process_group: info.process_group,
        foreground_group: info.foreground_group,
        executable: executable.to_string_lossy().into_owned(),
        argv: argv.values,
        environment: Some(argv.environment),
        cwd,
        evidence_paths,
    }))
}
