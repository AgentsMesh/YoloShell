use super::{COMPLETION_TARGET_PANIC_DETAIL, FaultReapCompletion, PublicationState};
use std::any::Any;
use std::borrow::Cow;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[cfg(test)]
pub(crate) const INJECTED_PAYLOAD_DROP_PANIC: &str = "injected completion payload Drop panic";
#[cfg(test)]
pub(crate) const TRACE_HOOK_ALREADY_INSTALLED_DETAIL: &str =
    "completion panic trace hook is already installed";

#[cfg(test)]
pub(crate) type TraceHook = Box<dyn FnOnce()>;

#[cfg(test)]
thread_local! {
    static TRACE_HOOK: std::cell::RefCell<Option<TraceHook>> = const {
        std::cell::RefCell::new(None)
    };
}

impl FaultReapCompletion {
    pub(crate) fn publish_target_panic(&self, payload: Box<dyn Any + Send>) {
        self.finish_publication(PublicationState::TargetPanicked(Cow::Borrowed(
            COMPLETION_TARGET_PANIC_DETAIL,
        )));
        let detail = inspect_payload(payload.as_ref());
        dispose_payload(payload);
        match detail {
            Some(detail) => {
                trace_detail(&detail);
                self.replace_target_panic_detail(detail);
            }
            None => trace_generic(),
        }
    }

    pub(crate) fn replace_target_panic_detail(&self, detail: String) {
        let mut publication = self
            .shared
            .publication
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let PublicationState::TargetPanicked(current) = &mut *publication {
            *current = Cow::Owned(detail);
        }
    }
}

pub(crate) fn inspect_payload(payload: &(dyn Any + Send)) -> Option<String> {
    contain(|| {
        payload
            .downcast_ref::<&str>()
            .map(|value| (*value).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
    })
    .flatten()
}

pub(crate) fn dispose_payload(payload: Box<dyn Any + Send>) {
    let _ = contain(|| drop(payload));
}

pub(crate) fn trace_detail(detail: &str) {
    let _ = contain(|| {
        if run_trace_hook() {
            return;
        }
        tracing::error!(%detail, COMPLETION_TARGET_PANIC_DETAIL);
    });
}

pub(crate) fn trace_generic() {
    let _ = contain(|| {
        if run_trace_hook() {
            return;
        }
        tracing::error!(COMPLETION_TARGET_PANIC_DETAIL);
    });
}

#[cfg(test)]
pub(crate) fn install_trace_hook(hook: impl FnOnce() + 'static) {
    TRACE_HOOK.with(|slot| {
        assert!(
            slot.replace(Some(Box::new(hook))).is_none(),
            "{TRACE_HOOK_ALREADY_INSTALLED_DETAIL}"
        );
    });
}

#[cfg(test)]
pub(crate) fn run_trace_hook() -> bool {
    TRACE_HOOK.with(|slot| {
        let Some(hook) = slot.borrow_mut().take() else {
            return false;
        };
        hook();
        true
    })
}

#[cfg(not(test))]
pub(crate) fn run_trace_hook() -> bool {
    false
}

pub(crate) fn contain<Value>(operation: impl FnOnce() -> Value) -> Option<Value> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Some(value),
        Err(payload) => {
            // A secondary panic payload may also panic on Drop. Leaking this
            // fault-only allocation is the terminal containment boundary.
            std::mem::forget(payload);
            None
        }
    }
}

#[cfg(test)]
pub(crate) fn panic_on_drop_for_test() -> ! {
    std::panic::panic_any(PanicOnDropPayload)
}

#[cfg(test)]
pub(crate) struct PanicOnDropPayload;

#[cfg(test)]
impl Drop for PanicOnDropPayload {
    fn drop(&mut self) {
        panic!("{INJECTED_PAYLOAD_DROP_PANIC}");
    }
}
