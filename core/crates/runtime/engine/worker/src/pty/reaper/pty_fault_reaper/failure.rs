pub(crate) const EARLIER_OWNERSHIP_FAULT_DETAIL: &str = "earlier PTY ownership fault";

pub(crate) fn remember_terminal(failure: &mut Option<anyhow::Error>, terminal: anyhow::Error) {
    let Some(previous) = failure.take() else {
        *failure = Some(terminal);
        return;
    };
    let previous = super::super::terminal_reporting::error_text(previous);
    let terminal_detail = super::super::terminal_reporting::error_ref_text(&terminal);
    let detail = format!("{EARLIER_OWNERSHIP_FAULT_DETAIL}: {previous}; {terminal_detail}");
    *failure = Some(terminal.context(detail));
}
