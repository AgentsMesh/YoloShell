use crate::process::{DetachedStandardError, ProcessIdentity};
use std::path::Path;

const TEST_RUNTIME_COMMAND: &str = "/bin/sleep";
const TEST_RUNTIME_DURATION_SECONDS: &str = "300";

pub(super) struct TestRuntimeChild {
    child: Option<std::process::Child>,
    identity: ProcessIdentity,
}

impl TestRuntimeChild {
    pub(super) fn spawn() -> Self {
        let child = crate::process::spawn_detached(
            Path::new(TEST_RUNTIME_COMMAND),
            &[TEST_RUNTIME_DURATION_SECONDS.into()],
            DetachedStandardError::Null,
        )
        .unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        Self {
            child: Some(child),
            identity,
        }
    }

    pub(super) fn identity(&self) -> ProcessIdentity {
        self.identity
    }

    pub(super) fn settle(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for TestRuntimeChild {
    fn drop(&mut self) {
        self.settle();
    }
}
