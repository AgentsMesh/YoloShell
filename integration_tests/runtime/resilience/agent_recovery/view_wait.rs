use super::*;

const SIDECAR_DIAGNOSTIC_MAX_BYTES: u64 = 64 * 1024;
const SIDECAR_TOO_LARGE: &str = "sidecar exceeds diagnostic size limit";
const VIEW_READ_FAILED: &str = "agent recovery view read failed";

pub(super) async fn wait_for_view(
    client: &Client,
    id: Uuid,
    ready: impl Fn(&TerminalView) -> bool,
    tracker: Option<&Harness>,
    diagnostics: &Harness,
) -> TerminalView {
    let deadline = tokio::time::Instant::now() + AGENT_STATE_SETTLEMENT_TIMEOUT;
    loop {
        let view = client.get_session(id).await.unwrap_or_else(|error| {
            panic!(
                "{VIEW_READ_FAILED}: session={id}; error={error:?}; host_stderr={}",
                diagnostics.host_stderr_tail()
            )
        });
        if ready(&view) {
            if let Some(harness) = tracker {
                harness.track_view(&view);
            }
            return view;
        }
        if tokio::time::Instant::now() >= deadline {
            report_timeout(client, id, diagnostics, view).await;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn report_timeout(client: &Client, id: Uuid, harness: &Harness, view: TerminalView) -> ! {
    let processes = view
        .runtime
        .as_ref()
        .and_then(|runtime| RuntimeProcessSource::new([]).descendants(runtime.child_pid).ok());
    let output = client.attach(id, 0).await.ok().map(|attachment| {
        attachment
            .initial
            .chunks
            .into_iter()
            .flat_map(|chunk| chunk.data)
            .collect::<Vec<_>>()
    });
    let sidecars = recovery_sidecars(harness, id);
    panic!(
        "agent state did not converge: view={view:#?}, processes={processes:#?}, output={}, host_stderr={}, sidecars={sidecars:#?}",
        String::from_utf8_lossy(output.as_deref().unwrap_or_default()),
        harness.host_stderr_tail()
    );
}

fn recovery_sidecars(harness: &Harness, id: Uuid) -> Vec<(PathBuf, Option<String>)> {
    let mut paths = std::fs::read_dir(harness.paths().session_dir(id))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_type().ok()?.is_file().then_some(entry.path()))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| (path.clone(), diagnostic_contents(&path)))
        .collect()
}

fn diagnostic_contents(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > SIDECAR_DIAGNOSTIC_MAX_BYTES {
        return Some(SIDECAR_TOO_LARGE.to_owned());
    }
    std::fs::read_to_string(path).ok()
}
