pub(crate) async fn terminate_process_scope(scope: process::PtyProcessScope, force: bool) -> Result<()> {
    let mut tracked = HashSet::new();
    refresh_scope(&scope, &mut tracked).await?;
    if force {
        return force_scope(&scope, &mut tracked).await;
    }
    let mut signaled = HashSet::new();
    signal_new_targets(&scope, &tracked, &mut signaled, libc::SIGTERM)?;
    let deadline = tokio::time::Instant::now() + GRACEFUL_TERMINATION_TIMEOUT;
    loop {
        retain_live(&mut tracked);
        if tracked.is_empty() {
            refresh_scope(&scope, &mut tracked).await?;
            if tracked.is_empty() {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return force_scope(&scope, &mut tracked).await;
        }
        tokio::time::sleep(PROCESS_SCOPE_REFRESH_INTERVAL).await;
        refresh_scope(&scope, &mut tracked).await?;
        signal_new_targets(&scope, &tracked, &mut signaled, libc::SIGTERM)?;
    }
}

async fn terminate_submitted_process_scope(
    state: &Arc<WorkerState>,
    force: bool,
) -> Result<()> {
    let root = state.pty_root_identity();
    let scope = run_process_query(move || process::PtyProcessScope::capture(root)).await?;
    state
        .control_test_hooks
        .terminate_before_delivery
        .pause_if_armed()
        .await;
    terminate_process_scope(scope, force).await
}

async fn force_scope(
    scope: &process::PtyProcessScope,
    tracked: &mut HashSet<process::ProcessIdentity>,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + FORCE_TERMINATION_TIMEOUT;
    loop {
        refresh_scope(scope, tracked).await?;
        signal_targets(scope, tracked, libc::SIGKILL)?;
        retain_live(tracked);
        if tracked.is_empty() {
            refresh_scope(scope, tracked).await?;
            if tracked.is_empty() {
                return Ok(());
            }
        }
        anyhow::ensure!(
            tokio::time::Instant::now() < deadline,
            "PTY process identities survived forced termination: {tracked:?}"
        );
        tokio::time::sleep(GRACEFUL_TERMINATION_POLL_INTERVAL).await;
    }
}

async fn refresh_scope(
    scope: &process::PtyProcessScope,
    tracked: &mut HashSet<process::ProcessIdentity>,
) -> Result<()> {
    let anchors = tracked.clone();
    let scope = scope.clone();
    let found = run_process_query(move || scope.identities(&anchors)).await?;
    tracked.extend(found);
    Ok(())
}

fn signal_new_targets(
    scope: &process::PtyProcessScope,
    tracked: &HashSet<process::ProcessIdentity>,
    signaled: &mut HashSet<process::ProcessIdentity>,
    signal: i32,
) -> Result<()> {
    let new = tracked.difference(signaled).copied().collect();
    signal_targets(scope, &new, signal)?;
    signaled.extend(new);
    Ok(())
}

fn signal_targets(
    scope: &process::PtyProcessScope,
    tracked: &HashSet<process::ProcessIdentity>,
    signal: i32,
) -> Result<()> {
    let mut targets = tracked.iter().copied().collect::<Vec<_>>();
    targets.sort_by_key(|target| target.pid == scope.root_for_test().pid);
    for target in targets {
        let _ = target.signal(signal)?;
    }
    Ok(())
}

fn retain_live(tracked: &mut HashSet<process::ProcessIdentity>) {
    tracked.retain(|identity| identity.matches());
}
