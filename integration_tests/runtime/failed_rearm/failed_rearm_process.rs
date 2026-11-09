use anyhow::{Context, Result, ensure};
use yolo_host::failed_rearm_process_fixture::Boundary;

#[path = "failed_rearm_process_child.rs"]
mod child;
#[path = "failed_rearm_process_constants.rs"]
mod constants;
use constants::*;

#[test]
fn failed_resume_rearm_survives_every_abrupt_process_boundary() -> Result<()> {
    for boundary in Boundary::ALL {
        run_boundary(boundary)
            .with_context(|| format!("{BOUNDARY_CONTEXT}: {}", boundary.as_str()))?;
    }
    Ok(())
}

fn run_boundary(boundary: Boundary) -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().to_owned();
    let home = root.join(FIXTURE_HOME_DIRECTORY);
    let evidence = child::prepare(&home)?;
    child::crash(&home, boundary, &evidence)?;
    child::assert_no_process_residue(&home)?;
    child::recover(&home, boundary, &evidence)?;
    child::assert_no_process_residue(&home)?;
    temporary.close()?;
    ensure!(!root.exists(), TEMPORARY_RESIDUE);
    Ok(())
}
