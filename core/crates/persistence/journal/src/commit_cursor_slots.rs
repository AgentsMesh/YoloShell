use super::*;

const FIRST_CURSOR_SLOT: usize = 0;
const SECOND_CURSOR_SLOT: usize = 1;

#[derive(Clone, Copy, Debug)]
pub(super) struct CursorSlotWrite {
    pub(super) offset: u64,
    pub(super) record: CursorRecord,
}

#[derive(Debug)]
pub(super) struct CursorStagePlan {
    pub(super) record: CursorRecord,
    pub(super) writes: Vec<CursorSlotWrite>,
}

#[derive(Clone, Copy, Debug)]
struct CursorSlots {
    records: [Option<CursorRecord>; RETAINED_CURSOR_SLOT_COUNT],
}

pub(super) fn read_active_cursor(
    file: &File,
    identity: FileIdentity,
    path: &Path,
) -> Result<CursorRecord, JournalError> {
    CursorSlots::read(file, identity, path, false, false, false)?.active()
}

pub(super) fn plan_retained_stage(
    file: &File,
    identity: FileIdentity,
    path: &Path,
    current: CursorRecord,
    committed_bytes: u64,
) -> Result<CursorStagePlan, JournalError> {
    let slots = CursorSlots::read(file, identity, path, true, true, true)?;
    let mut revision = slots
        .records
        .iter()
        .flatten()
        .filter(|record| record.generation == current.generation)
        .map(|record| record.publication_revision)
        .chain(std::iter::once(current.publication_revision))
        .max()
        .ok_or(JournalError::InvalidFormat)?;
    let matching = slots
        .records
        .map(|record| record.filter(|candidate| candidate.generation == current.generation));
    let targets = match matching {
        [Some(first), Some(second)] => vec![if first.publication_revision
            <= second.publication_revision
        {
            FIRST_CURSOR_SLOT
        } else {
            SECOND_CURSOR_SLOT
        }],
        [Some(_), None] => vec![SECOND_CURSOR_SLOT],
        [None, Some(_)] => vec![FIRST_CURSOR_SLOT],
        [None, None] => {
            let first = slots.rewrite_slot()?;
            vec![first, other_slot(first)?]
        }
    };
    let mut writes = Vec::with_capacity(RETAINED_CURSOR_SLOT_COUNT);
    for target in targets {
        revision = revision
            .checked_add(CURSOR_REVISION_INCREMENT)
            .ok_or(JournalError::CursorRevisionExhausted)?;
        writes.push(CursorSlotWrite {
            offset: slot_offset(target)?,
            record: CursorRecord {
                generation: current.generation,
                committed_bytes,
                publication_revision: revision,
            },
        });
    }
    let record = writes
        .last()
        .map(|write| write.record)
        .ok_or(JournalError::InvalidFormat)?;
    Ok(CursorStagePlan { record, writes })
}

impl CursorSlots {
    fn read(
        file: &File,
        identity: FileIdentity,
        path: &Path,
        allow_partial_tail: bool,
        allow_mixed_generations: bool,
        tolerate_corrupt_slot: bool,
    ) -> Result<Self, JournalError> {
        let length = identity.length();
        let minimum = COMMIT_CURSOR_BYTES as u64;
        let maximum = RETAINED_CURSOR_BYTES as u64;
        if length < minimum
            || length > maximum
            || (!allow_partial_tail && length != minimum && length != maximum)
        {
            return Err(JournalError::InvalidFormat);
        }
        let records = [
            read_slot(file, length, FIRST_CURSOR_SLOT, tolerate_corrupt_slot)?,
            read_slot(file, length, SECOND_CURSOR_SLOT, tolerate_corrupt_slot)?,
        ];
        identity.verify_file_bounded(file, length, path, maximum)?;
        let slots = Self { records };
        slots.validate(allow_mixed_generations)?;
        Ok(slots)
    }

    fn active(self) -> Result<CursorRecord, JournalError> {
        self.records
            .into_iter()
            .flatten()
            .max_by_key(|record| record.publication_revision)
            .ok_or(JournalError::InvalidFormat)
    }

    fn rewrite_slot(self) -> Result<usize, JournalError> {
        match self.records {
            [Some(_), None] => Ok(SECOND_CURSOR_SLOT),
            [None, Some(_)] => Ok(FIRST_CURSOR_SLOT),
            [Some(first), Some(second)] => Ok(
                if first.publication_revision <= second.publication_revision {
                    FIRST_CURSOR_SLOT
                } else {
                    SECOND_CURSOR_SLOT
                },
            ),
            [None, None] => Err(JournalError::InvalidFormat),
        }
    }

    fn validate(self, allow_mixed_generations: bool) -> Result<(), JournalError> {
        match self.records {
            [None, None] => Err(JournalError::InvalidFormat),
            [Some(first), Some(second)] => {
                let mixed = first.generation != second.generation;
                let conflicting = !mixed
                    && first.publication_revision == second.publication_revision
                    && first != second;
                if conflicting || (mixed && !allow_mixed_generations) {
                    Err(JournalError::InvalidFormat)
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
}

fn other_slot(index: usize) -> Result<usize, JournalError> {
    match index {
        FIRST_CURSOR_SLOT => Ok(SECOND_CURSOR_SLOT),
        SECOND_CURSOR_SLOT => Ok(FIRST_CURSOR_SLOT),
        _ => Err(JournalError::InvalidFormat),
    }
}

#[path = "commit_cursor_slot_read.rs"]
mod commit_cursor_slot_read;
pub(crate) use commit_cursor_slot_read::{read_slot, slot_offset};

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/commit_cursor_slots_tests.rs"]
mod tests;
