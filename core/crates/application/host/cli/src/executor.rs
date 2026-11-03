use super::Command;
use tokio::runtime::{Builder, Runtime};

const HOST_SCHEDULER_THREADS: usize = 4;
const SESSION_SCHEDULER_THREADS: usize = 2;

pub(super) fn build(command: &Command) -> std::io::Result<Runtime> {
    let scheduler_threads = match command {
        Command::Serve { .. } => Some(HOST_SCHEDULER_THREADS),
        Command::SessionWorker { .. } | Command::CustodyGuardian { .. } => {
            Some(SESSION_SCHEDULER_THREADS)
        }
        _ => None,
    };
    let mut builder = match scheduler_threads {
        Some(threads) => {
            let mut builder = Builder::new_multi_thread();
            // Each session worker owns its executor; machine-sized pools multiply with runtimes.
            builder.worker_threads(threads);
            builder
        }
        None => Builder::new_current_thread(),
    };
    builder.enable_all().build()
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/executor_tests.rs"]
mod tests;
