use std::cell::Cell;

pub(super) const GONE_AUTHORITY_FAILURE_DETAIL: &str =
    "injected post-seal process authority failure";

thread_local! {
    static FAIL_NEXT: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn fail_next() {
    FAIL_NEXT.with(|failure| {
        assert!(!failure.replace(true), "{GONE_AUTHORITY_FAILURE_DETAIL}");
    });
}

pub(super) fn fail_if_requested() -> anyhow::Result<()> {
    if FAIL_NEXT.with(|failure| failure.replace(false)) {
        anyhow::bail!(GONE_AUTHORITY_FAILURE_DETAIL);
    }
    Ok(())
}
