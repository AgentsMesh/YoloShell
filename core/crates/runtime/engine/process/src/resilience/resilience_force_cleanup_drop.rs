use super::{
    ResilienceFreezingProcessTree, ResilienceFrozenProcessTree, ResilienceKillPendingProcessTree,
    RetainedProcessCleanup, terminal,
};

const CLEANUP_DROP_FAILURE_DETAIL: &str = "retained process cleanup failed during drop";
const CLEANUP_DROP_PANIC_DETAIL: &str = "retained process cleanup panicked during drop";
const CLEANUP_CAPTURE_PANIC_DETAIL: &str = "retained process target capture panicked during drop";

fn drop_cleanup(cleanup: &mut Option<RetainedProcessCleanup>) {
    let Some(mut cleanup) = cleanup.take() else {
        return;
    };
    let failed = match terminal::contain(|| cleanup.run()) {
        Some(Ok(())) => false,
        Some(Err(error)) => {
            terminal::report_error(error, CLEANUP_DROP_FAILURE_DETAIL);
            true
        }
        None => {
            terminal::report_message(CLEANUP_DROP_PANIC_DETAIL);
            true
        }
    };
    if failed {
        let _ = terminal::contain(|| cleanup.settle_contained());
    }
}

macro_rules! cleanup_on_drop {
    ($state:ty) => {
        impl Drop for $state {
            fn drop(&mut self) {
                let captured = terminal::contain(|| {
                    if let (Some(inner), Some(cleanup)) = (&self.inner, &mut self.cleanup) {
                        cleanup.absorb(inner.resilience_retained_targets());
                    }
                });
                if captured.is_none() {
                    terminal::report_message(CLEANUP_CAPTURE_PANIC_DETAIL);
                }
                drop_cleanup(&mut self.cleanup);
            }
        }
    };
}

cleanup_on_drop!(ResilienceFreezingProcessTree);
cleanup_on_drop!(ResilienceFrozenProcessTree);
cleanup_on_drop!(ResilienceKillPendingProcessTree);
