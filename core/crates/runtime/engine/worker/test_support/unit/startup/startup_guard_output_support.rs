use crate::journal::Journal;
use crate::worker::pty_output_progress::ExclusivePtyOutput;
use portable_pty::MasterPty;
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const TEST_JOURNAL_EXTENSION: &str = "journal";

pub(super) fn capture_test_output(master: &dyn MasterPty, marker: &Path) -> ExclusivePtyOutput {
    let journal = Arc::new(Mutex::new(
        Journal::open(
            marker.with_extension(TEST_JOURNAL_EXTENSION),
            Uuid::new_v4(),
        )
        .unwrap(),
    ));
    ExclusivePtyOutput::capture(master, journal).unwrap()
}
