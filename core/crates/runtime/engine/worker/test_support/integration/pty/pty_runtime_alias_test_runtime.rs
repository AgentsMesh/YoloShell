use std::sync::OnceLock;

const TEST_RUNTIME_WORKER_THREADS: usize = 1;
const TEST_RUNTIME_BUILD_INVARIANT: &str = "build the PTY alias closure test runtime";

pub(super) fn handle() -> tokio::runtime::Handle {
    tokio::runtime::Handle::try_current().unwrap_or_else(|_| runtime().handle().clone())
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(TEST_RUNTIME_WORKER_THREADS)
            .enable_time()
            .build()
            .expect(TEST_RUNTIME_BUILD_INVARIANT)
    })
}
