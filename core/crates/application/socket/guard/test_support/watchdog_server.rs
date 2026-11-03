use super::constants::*;
use super::custody::{DiscoveryCustody, RootCustody, TreeCustody};
use super::protocol::{
    read_capability, read_tracked_root, write_completion, write_identity, write_root,
};
use anyhow::{Context, Result, ensure};
use std::io::BufReader;
use std::sync::mpsc;

pub(super) fn run_watchdog() -> Result<std::process::ExitCode> {
    let capability = super::protocol::ControlCapability::generate();
    let discovery = DiscoveryCustody::adopt_inherited()?;
    let mut root = RootCustody::create()?;
    write_root(
        &mut std::io::stdout(),
        &capability,
        discovery.protocol(),
        discovery.reader_handle(),
        discovery.writer_handle(),
        root.path(),
    )?;
    let mut control = BufReader::new(std::io::stdin());
    let (identity, descriptor, origin) = read_tracked_root(&mut control, &capability)?;
    let mut tree = TreeCustody::capture(
        identity,
        discovery.into_verified(identity, descriptor)?,
        origin,
    )?;
    write_identity(&mut std::io::stdout(), FRAME_TRACKED, &capability, identity)?;

    let (finished, receiver) = mpsc::sync_channel(CONTROL_CHANNEL_CAPACITY);
    let finish_capability = capability.clone();
    let reader = std::thread::spawn(move || {
        let result = read_capability(&mut control, FRAME_FINISH, &finish_capability);
        let _ = finished.send(result);
    });
    let (finish, refresh_failure) = monitor(&mut tree, &receiver);
    let _ = reader.join();

    let mut failures = Vec::new();
    record(&mut failures, finish);
    if let Some(error) = refresh_failure {
        failures.push(error);
    }
    record(&mut failures, tree.settle());
    record(&mut failures, root.finalize());
    let detail = (!failures.is_empty()).then(|| failures.join(FAILURE_SEPARATOR));
    record(
        &mut failures,
        write_completion(&mut std::io::stdout(), &capability, detail.as_deref())
            .context(WATCHDOG_COMPLETION_WRITE_DETAIL),
    );
    ensure!(failures.is_empty(), failures.join(FAILURE_SEPARATOR));
    Ok(std::process::ExitCode::SUCCESS)
}

fn monitor(
    tree: &mut TreeCustody,
    receiver: &mpsc::Receiver<Result<()>>,
) -> (Result<()>, Option<String>) {
    let mut failure = None;
    loop {
        match receiver.try_recv() {
            Ok(result) => return (result, failure),
            Err(mpsc::TryRecvError::Disconnected) => {
                return (Err(anyhow::anyhow!(WATCHDOG_PROTOCOL_DETAIL)), failure);
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        let refreshed = tree.refresh().and_then(|()| {
            #[cfg(feature = "adversarial-test-hooks")]
            tree.acknowledge_historical_witness()?;
            Ok(())
        });
        if let Err(error) = refreshed
            && failure.is_none()
        {
            failure = Some(format!("{WATCHDOG_REFRESH_DETAIL}: {error:#}"));
        }
        std::thread::sleep(PROCESS_POLL_INTERVAL);
    }
}

fn record(failures: &mut Vec<String>, result: Result<()>) {
    if let Err(error) = result {
        failures.push(format!("{error:#}"));
    }
}
