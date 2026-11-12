use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const PROGRESS_PROBE_BUDGET: Duration = Duration::from_millis(250);
const PROGRESS_PROBE_STARVED: &str = "snapshot decode starved the async runtime";

#[cfg(feature = "yolo-unit-tests")]
#[tokio::test(flavor = "current_thread")]
async fn bounded_decode_work_does_not_starve_async_progress() {
    let entered = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let worker_entered = entered.clone();
    let worker_release = release.clone();
    let decode = tokio::spawn(run_in_background(move || {
        worker_entered.store(true, Ordering::Release);
        let deadline = Instant::now() + PROGRESS_PROBE_BUDGET;
        while !worker_release.load(Ordering::Acquire) {
            anyhow::ensure!(Instant::now() < deadline, PROGRESS_PROBE_STARVED);
            std::thread::yield_now();
        }
        Ok(())
    }));
    tokio::time::timeout(PROGRESS_PROBE_BUDGET, async {
        while !entered.load(Ordering::Acquire) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect(PROGRESS_PROBE_STARVED);

    release.store(true, Ordering::Release);

    decode.await.unwrap().unwrap();
}
