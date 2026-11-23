use tokio::runtime::{Builder, Runtime};

pub(super) fn build() -> std::io::Result<Runtime> {
    Builder::new_current_thread().enable_all().build()
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/executor_tests.rs"]
mod tests;
