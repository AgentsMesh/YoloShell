pub async fn crash(home: &Path, boundary: Boundary, evidence: &PreparedEvidence) -> Result<()> {
    evidence.validate_home(home)?;
    ensure!(
        configured_boundary()? == Some(boundary),
        FIXTURE_BOUNDARY_MISMATCH
    );
    let host = Host::open(fixture_config(home))?;
    ensure!(
        host.controller_generation > evidence.controller_generation,
        FIXTURE_GENERATION_MISMATCH
    );
    let _result = host
        .resume_session_locked(evidence.terminal_id, ResumeSelection::ExplicitRecreate)
        .await;
    bail!(FIXTURE_CRASH_NOT_DELIVERED)
}
