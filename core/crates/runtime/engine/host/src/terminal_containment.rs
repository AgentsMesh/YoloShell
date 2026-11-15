use futures::FutureExt;
use std::fmt::Debug;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};

const DEBUG_DETAIL_FALLBACK: &str = "watchdog result formatting panicked";
const DROP_GUARD_EMPTY: &str = "terminal drop guard value is present";
const ERROR_DETAIL_FALLBACK: &str = "host error formatting panicked";
const PANIC_DETAIL_FALLBACK: &str = "host task panicked without a reportable payload";

pub(super) struct DropGuard<Value> {
    value: Option<Value>,
}

impl<Value> DropGuard<Value> {
    pub(super) fn new(value: Value) -> Self {
        Self { value: Some(value) }
    }

    pub(super) fn get_mut(&mut self) -> &mut Value {
        self.value.as_mut().expect(DROP_GUARD_EMPTY)
    }
}

impl<Value> Drop for DropGuard<Value> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            discard(value);
        }
    }
}

pub(super) fn catch<Value>(operation: impl FnOnce() -> Value) -> Result<Value, ()> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            dispose_panic(payload);
            Err(())
        }
    }
}

pub(super) fn catch_future<Fut>(future: Fut) -> impl Future<Output = Result<Fut::Output, ()>>
where
    Fut: Future,
{
    let caught = catch_future_detail(future);
    async move { caught.await.map_err(drop) }
}

pub(super) fn catch_future_detail<Fut>(
    future: Fut,
) -> impl Future<Output = Result<Fut::Output, String>>
where
    Fut: Future,
{
    let caught = Box::pin(AssertUnwindSafe(future).catch_unwind());
    async move {
        let mut guarded = DropGuard::new(caught);
        match guarded.get_mut().as_mut().await {
            Ok(value) => Ok(value),
            Err(payload) => Err(panic_detail(payload)),
        }
    }
}

pub(super) fn debug_detail<Value: Debug>(value: Value) -> String {
    let detail =
        catch(|| format!("{value:?}")).unwrap_or_else(|()| DEBUG_DETAIL_FALLBACK.to_owned());
    discard(value);
    detail
}

pub(super) fn error_detail(error: anyhow::Error) -> String {
    let detail =
        catch(|| format!("{error:#}")).unwrap_or_else(|()| ERROR_DETAIL_FALLBACK.to_owned());
    discard(error);
    detail
}

pub(super) fn panic_detail(payload: Box<dyn std::any::Any + Send>) -> String {
    let detail = catch(|| {
        payload
            .downcast_ref::<&str>()
            .map(|value| (*value).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
    })
    .ok()
    .flatten()
    .unwrap_or_else(|| PANIC_DETAIL_FALLBACK.to_owned());
    discard(payload);
    detail
}

pub(super) fn discard<Value>(value: Value) -> bool {
    catch(|| drop(value)).is_ok()
}

pub(super) fn trace(operation: impl FnOnce()) {
    let _ = catch(operation);
}

fn dispose_panic(payload: Box<dyn std::any::Any + Send>) {
    if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        std::mem::forget(secondary);
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/terminal_containment_core_tests.rs"]
mod tests;
