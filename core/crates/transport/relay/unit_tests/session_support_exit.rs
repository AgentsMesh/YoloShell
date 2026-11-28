const EXIT_CODE: i32 = 0;
const EXIT_OBSERVED_AT_MS: i64 = 1;
const EXIT_OUTPUT_SEQUENCE: u64 = 0;

pub(crate) fn exit_event(terminal_id: Uuid) -> EventEnvelope {
    exit_event_at(terminal_id, EXIT_OUTPUT_SEQUENCE)
}

pub(crate) fn exit_event_at(terminal_id: Uuid, last_output_sequence: u64) -> EventEnvelope {
    EventEnvelope::new(
        event::SESSION_EXITED,
        Some(terminal_id.to_string()),
        Some(test_runtime_epoch().to_string()),
        &natural_exit_at(last_output_sequence),
    )
    .unwrap()
}

pub(crate) fn natural_exit() -> ExitRecord {
    natural_exit_at(EXIT_OUTPUT_SEQUENCE)
}

pub(crate) fn natural_exit_at(last_output_sequence: u64) -> ExitRecord {
    ExitRecord {
        runtime_epoch: test_runtime_epoch(),
        code: Some(EXIT_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: EXIT_OBSERVED_AT_MS,
        last_output_sequence,
    }
}

pub(crate) fn completed_session(terminal_id: Uuid) -> TerminalView {
    let mut view = running_session(terminal_id);
    let runtime = view.runtime.as_mut().unwrap();
    runtime.status = TerminalRuntimeStatus::Exited;
    runtime.last_output_sequence = EXIT_OUTPUT_SEQUENCE;
    runtime.exit = Some(natural_exit());
    view.completed_attachment = Some(crate::model::CompletedAttachmentView {
        runtime_epoch: test_runtime_epoch(),
    });
    view
}
