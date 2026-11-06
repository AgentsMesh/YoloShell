use super::*;
use std::fmt::Display;
use std::future::Future;

const DISPLAY_DETAIL_FALLBACK: &str = "secondary-custody error formatting panicked";
const PLATFORM_DROP_FAIL_STOP_DETAIL: &str =
    "Host could not safely destroy retired PTY secondary custody";

pub(super) fn catch_future_detail<Fut>(
    future: Fut,
) -> impl Future<Output = Result<Fut::Output, String>>
where
    Fut: Future,
{
    super::super::terminal_containment::catch_future_detail(future)
}

pub(super) fn error_detail(error: anyhow::Error) -> String {
    super::super::terminal_containment::error_detail(error)
}

pub(super) fn display_detail(value: &impl Display) -> String {
    super::super::terminal_containment::catch(|| value.to_string())
        .unwrap_or_else(|()| DISPLAY_DETAIL_FALLBACK.to_owned())
}

pub(super) fn owned_display_detail<Value: Display>(value: Value) -> String {
    let detail = display_detail(&value);
    super::super::terminal_containment::discard(value);
    detail
}

pub(super) fn fail_stop(drain: &HostRequestDrain, detail: String) {
    let primary = detail.clone();
    let _ = super::super::terminal_containment::catch(|| drain.fail_stop(primary));
    let _ = super::super::terminal_containment::catch(|| {
        tracing::subscriber::with_default(tracing::subscriber::NoSubscriber::default(), || {
            drain.fail_stop(detail);
        });
    });
}

pub(super) fn discard_after_retirement<Value>(value: Value, drain: &HostRequestDrain) {
    if !super::super::terminal_containment::discard(value) {
        fail_stop(drain, PLATFORM_DROP_FAIL_STOP_DETAIL.to_owned());
    }
}
