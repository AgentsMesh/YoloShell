use yolo_wire::field;

fn redacted_authority() -> &'static str {
    REDACTED_CREATION_AUTHORITY
}

impl fmt::Debug for CreateTerminal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreateTerminal")
            .field(field::CREATION_TOKEN, &redacted_authority())
            .field(field::NAME, &self.name)
            .field(field::CWD, &self.cwd)
            .field(field::COLS, &self.cols)
            .field(field::ROWS, &self.rows)
            .field(field::DATA, &redacted_authority())
            .field("context", &self.context)
            .finish()
    }
}

impl fmt::Debug for CreateTerminalResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreateTerminalResult")
            .field(field::TERMINAL_ID, &self.terminal_id)
            .field(field::CREATION_CAPABILITY, &redacted_authority())
            .finish()
    }
}

macro_rules! lifecycle_debug {
    ($request:ty, $name:literal, $($member:ident => $field:path),* $(,)?) => {
        impl fmt::Debug for $request {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut output = formatter.debug_struct($name);
                $(output.field($field, &self.$member);)*
                output
                    .field(field::CREATION_CAPABILITY, &redacted_authority())
                    .finish()
            }
        }
    };
}

lifecycle_debug!(
    CreationCleanupArchiveRequest,
    "CreationCleanupArchiveRequest",
    terminal_id => field::TERMINAL_ID,
);
lifecycle_debug!(
    CreationCleanupTerminateRequest,
    "CreationCleanupTerminateRequest",
    terminal_id => field::TERMINAL_ID,
    force => field::FORCE,
);
lifecycle_debug!(
    CreationCleanupDiscardRequest,
    "CreationCleanupDiscardRequest",
    terminal_id => field::TERMINAL_ID,
    force => field::FORCE,
);

impl fmt::Debug for FinalizeCreationRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FinalizeCreationRequest")
            .field(field::CREATION_TOKEN, &redacted_authority())
            .field(field::TERMINAL_ID, &self.terminal_id)
            .field(field::CREATION_CAPABILITY, &redacted_authority())
            .finish()
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/creation_debug_tests.rs"]
mod debug_tests;
