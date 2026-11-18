use super::{
    support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY},
    tui_queries::{constants, driver::Driver},
    tui_workspaces::support::lease,
};
use crate::{Harness, runtime_ref_from_view};
use std::time::{Duration, Instant};
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::LeaseObservation;

const SOURCE_NAME: &str = constants::FIRST_NAME;
const CONTROL: &str = " | Control: this UI";
const READY_PREFIX: &str = "YOLO_LATENCY_";
const READY_SUFFIX: &str = "READY";
const ECHO_PREFIX: &str = "YOLO_LATENCY_ECHO_";
const NOISE_TRIGGER: u8 = b'!';
const NOISE_STOP: u8 = b'@';
const SAMPLE_COUNT: usize = 64;
const LATENCY_LIMIT: Duration = Duration::from_secs(3);
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(10);
const SNAPSHOT_POLL: Duration = Duration::from_millis(10);

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_input_latency_records_idle_and_busy_private_pty_echo() {
    let host = Harness::start().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let hub = RunningHub::start(
        &host,
        &root.path().canonicalize().unwrap().join(STATE_DIRECTORY),
    )
    .await;
    let (source, target) = prepare_source(&host, hub.hub).await;
    let driver_host = Harness::start().await;
    let mut driver = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
    driver.open(SOURCE_NAME, CONTROL).await;
    let before = lease(&hub, target).await.unwrap();
    let idle = measure(&mut driver, false, 1).await;
    let busy = measure(&mut driver, true, SAMPLE_COUNT + 1).await;
    report("idle", &idle);
    report("busy", &busy);
    assert_complete(&idle);
    assert_complete(&busy);
    assert_same_writer(&before, &lease(&hub, target).await.unwrap());
    driver.finish().await;
    host.client()
        .terminate(source.manifest.terminal_id, false)
        .await
        .unwrap();
    hub.stop().await;
}

async fn prepare_source(
    host: &Harness,
    hub: yolo_hub_identity::HubId,
) -> (yolo_runtime::TerminalView, RuntimeTarget) {
    let source = host.create_cat(SOURCE_NAME).await;
    let runtime = runtime_ref_from_view(&source);
    let target = RuntimeTarget {
        address: TerminalAddress {
            hub_id: hub,
            terminal_id: TerminalId::from_uuid(runtime.terminal_id).unwrap(),
        },
        runtime_epoch: RuntimeEpoch::from_uuid(runtime.runtime_epoch).unwrap(),
    };
    let client = host.client();
    let grant = client
        .acquire_writer(runtime, SOURCE_NAME, Some(5_000))
        .await
        .unwrap();
    let ready_tail = uuid::Uuid::new_v4().simple().to_string();
    let ready = format!("{READY_PREFIX}{READY_SUFFIX}_{ready_tail}");
    let command = format!(
        "stty -echo -icanon min 1 time 0; count=0; noise_pid=; printf '%s%s%s\\n' '{READY_PREFIX}' '{READY_SUFFIX}_' '{ready_tail}'; while :; do c=$(dd bs=1 count=1 2>/dev/null) || break; if [ \"$c\" = \"!\" ]; then (while true; do printf '\\r\\nYOLO_LATENCY_NOISE\\r\\n'; sleep 0.02; done) & noise_pid=$!; elif [ \"$c\" = \"@\" ]; then kill \"$noise_pid\" 2>/dev/null || true; else count=$((count + 1)); printf '\\r\\n%s%s%d\\r\\n' '{ECHO_PREFIX}' \"$c\" \"$count\"; fi; done\n"
    );
    client
        .input(
            runtime,
            grant.generation,
            &grant.writer_capability,
            command.into_bytes(),
        )
        .await
        .unwrap();
    wait_snapshot(&client, source.manifest.terminal_id, &ready).await;
    client.release_writer(&grant, runtime).await.unwrap();
    (source, target)
}

async fn wait_snapshot(client: &yolo_client::Client, terminal_id: uuid::Uuid, marker: &str) {
    tokio::time::timeout(SNAPSHOT_TIMEOUT, async {
        loop {
            if let Ok(snapshot) = client.snapshot(terminal_id).await
                && String::from_utf8_lossy(&snapshot.data).contains(marker)
            {
                return;
            }
            tokio::time::sleep(SNAPSHOT_POLL).await;
        }
    })
    .await
    .unwrap();
}

async fn measure(driver: &mut Driver, noisy: bool, first_count: usize) -> Vec<Duration> {
    if noisy {
        driver.send(&[NOISE_TRIGGER]).await;
    }
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for offset in 0..SAMPLE_COUNT {
        let count = first_count + offset;
        let key = b'a' + (offset % 26) as u8;
        let marker = format!("{ECHO_PREFIX}{}{}", key as char, count);
        let started = Instant::now();
        driver.send(&[key]).await;
        driver.wait(|text| text.contains(&marker)).await;
        samples.push(started.elapsed());
    }
    if noisy {
        driver.send(&[NOISE_STOP]).await;
    }
    samples
}

fn assert_complete(samples: &[Duration]) {
    assert_eq!(samples.len(), SAMPLE_COUNT);
    assert!(samples.iter().all(|sample| *sample < LATENCY_LIMIT));
}

fn report(label: &str, samples: &[Duration]) {
    let mut values = samples.iter().map(Duration::as_micros).collect::<Vec<_>>();
    values.sort_unstable();
    let p50 = values[values.len() / 2] as f64 / 1_000.0;
    let p95 = values[values.len() * 95 / 100] as f64 / 1_000.0;
    let max = values[values.len() - 1] as f64 / 1_000.0;
    println!(
        "input_latency {label} count={} p50_ms={p50:.3} p95_ms={p95:.3} max_ms={max:.3}",
        values.len()
    );
}

fn assert_same_writer(before: &LeaseObservation, after: &LeaseObservation) {
    assert_eq!(before.fields().generation, after.fields().generation);
    assert_eq!(before.fields().holder_id, after.fields().holder_id);
    assert_eq!(
        before.fields().controller_instance_id,
        after.fields().controller_instance_id
    );
}
