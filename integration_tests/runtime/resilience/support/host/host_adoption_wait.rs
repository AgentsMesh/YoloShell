use super::snapshot::inspect_runtime;
use super::*;
use std::future::Future;
use yolo_runtime::model::HostStatus;

const ADOPTION_HOST_EXIT_DETAIL: &str = "replacement Host exited before runtime adoption";
const ADOPTION_HOST_STATUS_DETAIL: &str = "inspect replacement Host during runtime adoption";
const ADOPTION_HOST_CHANGED_DETAIL: &str = "replacement Host identity changed during adoption";
const ADOPTION_GENERATION_DETAIL: &str = "replacement Host generation cannot adopt runtime";
const ADOPTION_VIEW_DETAIL: &str = "inspect runtime view during Host adoption";
pub(super) const ADOPTION_TIMEOUT_DETAIL: &str =
    "replacement Host did not adopt all surviving runtimes";

pub(in crate::support) async fn wait_for_runtime_adoptions(
    home: &Path,
    host: &mut HostGuard,
    status: HostStatus,
    expected: Vec<RuntimeAdoptionExpectation>,
) -> Result<(), String> {
    if expected.is_empty() {
        return Ok(());
    }
    let client = Client::new(Paths::new(home).host_socket().unwrap());
    host.verify_ready_status(&status)
        .map_err(|error| format!("{ADOPTION_HOST_CHANGED_DETAIL}: {error:#}"))?;
    validate_successor_generation(&expected, status.controller_generation)?;
    wait_for_adoption_progress(
        expected.len(),
        HOST_READY_TIMEOUT,
        HOST_READY_POLL_INTERVAL,
        || host_preflight(host),
        || {
            let client = client.clone();
            let expected = expected.clone();
            let status = status.clone();
            async move { inspect_round(&client, &status, &expected).await }
        },
    )
    .await
}

fn validate_successor_generation(
    expected: &[RuntimeAdoptionExpectation],
    successor: u64,
) -> Result<(), String> {
    if expected
        .iter()
        .all(|runtime| successor > runtime.record().controller_generation)
    {
        return Ok(());
    }
    Err(ADOPTION_GENERATION_DETAIL.into())
}

fn host_preflight(host: &mut HostGuard) -> Result<(), String> {
    match host.try_wait() {
        Ok(Some(status)) => Err(format!(
            "{ADOPTION_HOST_EXIT_DETAIL}: {status}; stderr tail: {}",
            host.stderr_tail()
        )),
        Err(error) => Err(format!("{ADOPTION_HOST_STATUS_DETAIL}: {error}")),
        Ok(None) => Ok(()),
    }
}

async fn inspect_round(
    client: &Client,
    successor: &HostStatus,
    expected: &[RuntimeAdoptionExpectation],
) -> Result<AdoptionProgress, String> {
    let current_host = match client.status().await {
        Ok(status) => status,
        Err(error) => {
            return Ok(AdoptionProgress::Pending(format!(
                "{ADOPTION_HOST_STATUS_DETAIL}: {error:?}"
            )));
        }
    };
    if current_host.host_pid != successor.host_pid
        || current_host.controller_generation != successor.controller_generation
        || current_host.home != successor.home
    {
        return Err(ADOPTION_HOST_CHANGED_DETAIL.into());
    }
    for runtime in expected {
        match inspect_runtime(runtime, successor.controller_generation, || {
            client.get_session(runtime.record().terminal_id)
        })
        .await?
        {
            AdoptionProgress::Complete => {}
            pending => return Ok(pending),
        }
    }
    Ok(AdoptionProgress::Complete)
}

pub(super) fn pending_session_view_error(
    terminal_id: Uuid,
    error: impl std::fmt::Debug,
) -> AdoptionProgress {
    AdoptionProgress::Pending(format!(
        "{ADOPTION_PENDING_DETAIL}: {ADOPTION_VIEW_DETAIL}; terminal={terminal_id}; error={error:?}"
    ))
}

pub(super) async fn wait_for_adoption_progress<P, F, Fut>(
    expected_count: usize,
    timeout: Duration,
    poll_interval: Duration,
    mut preflight: P,
    mut probe: F,
) -> Result<(), String>
where
    P: FnMut() -> Result<(), String>,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<AdoptionProgress, String>>,
{
    if expected_count == usize::default() {
        return Ok(());
    }
    let deadline = tokio::time::Instant::now() + timeout;
    let mut pending = ADOPTION_PENDING_DETAIL.to_owned();
    loop {
        preflight()?;
        match tokio::time::timeout_at(deadline, probe()).await {
            Ok(Ok(AdoptionProgress::Complete)) => return Ok(()),
            Ok(Ok(AdoptionProgress::Pending(detail))) => pending = detail,
            Ok(Err(error)) => return Err(error),
            Err(_) => return Err(format!("{ADOPTION_TIMEOUT_DETAIL}: {pending}")),
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("{ADOPTION_TIMEOUT_DETAIL}: {pending}"));
        }
        let next = tokio::time::Instant::now() + poll_interval;
        tokio::time::sleep_until(std::cmp::min(next, deadline)).await;
    }
}
