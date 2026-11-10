pub async fn recover_and_verify(
    home: &Path,
    boundary: Boundary,
    evidence: &PreparedEvidence,
) -> Result<()> {
    let crash = verify_crash_state(home, boundary, evidence)?;
    let host = Host::open(fixture_config(home))?;
    ensure!(
        host.controller_generation > crash.transaction.launch.controller_generation
            && host.controller_generation > evidence.controller_generation,
        FIXTURE_GENERATION_MISMATCH
    );
    let result = host.reconcile().await;
    ensure!(
        result.unreachable == EXPECTED_RECONCILE_UNREACHABLE,
        FIXTURE_RECONCILE_MISMATCH
    );
    verify_recovered_state(&host, boundary, evidence, &crash).await?;
    drop(host);
    ensure!(!evidence.socket_path().exists(), FIXTURE_SOCKET_RESIDUE);
    Ok(())
}
