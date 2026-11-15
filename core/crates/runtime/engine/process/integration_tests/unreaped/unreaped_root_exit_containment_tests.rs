use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const PAYLOAD_DROP_PANIC: &str = "injected child process-id payload Drop panic";

#[derive(Clone, Debug)]
struct PanickingChild {
    payload_dropped: Arc<AtomicBool>,
}

impl portable_pty::ChildKiller for PanickingChild {
    fn kill(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(self.clone())
    }
}

impl portable_pty::Child for PanickingChild {
    fn try_wait(&mut self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        unreachable!()
    }

    fn wait(&mut self) -> std::io::Result<portable_pty::ExitStatus> {
        unreachable!()
    }

    fn process_id(&self) -> Option<u32> {
        std::panic::panic_any(PanicPayload(Arc::clone(&self.payload_dropped)))
    }
}

struct PanicPayload(Arc<AtomicBool>);

impl Drop for PanicPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn process_id_panic_payload_drop_is_contained_as_an_error() {
    let payload_dropped = Arc::new(AtomicBool::new(false));
    let child: OwnedPtyChild = Box::new(PanickingChild {
        payload_dropped: Arc::clone(&payload_dropped),
    });

    let result = child_process_id(&child);

    assert!(result.is_err());
    assert!(payload_dropped.load(Ordering::SeqCst));
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains(CHILD_PID_QUERY_PANIC_DETAIL)
    );
}
