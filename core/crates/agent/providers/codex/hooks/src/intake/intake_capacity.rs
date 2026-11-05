use super::constants::errors;
use super::intake::CodexHookIntake;
use anyhow::{Context, Result, ensure};

struct IntakeCapacity {
    entries: usize,
    occurrences: usize,
}

impl CodexHookIntake {
    pub(super) fn verify_occurrence_capacity(&self, staged: bool) -> Result<()> {
        let capacity = self.capacity()?;
        verify_limit(
            capacity.entries,
            super::intake_constants::MAX_DIRECTORY_ENTRIES,
            staged,
        )?;
        verify_limit(
            capacity.occurrences,
            super::intake_constants::MAX_PENDING_OCCURRENCES,
            staged,
        )
    }

    pub(super) fn verify_artifact_capacity(&self, staged: bool) -> Result<()> {
        verify_limit(
            self.capacity()?.entries,
            super::intake_constants::MAX_DIRECTORY_ENTRIES,
            staged,
        )
    }

    fn capacity(&self) -> Result<IntakeCapacity> {
        let mut stream =
            crate::paths::directory_stream::DirectoryStream::open(self.directory().descriptor())
                .context(errors::INTAKE_STORAGE)?;
        let mut entries = usize::default();
        let mut occurrences = usize::default();
        while let Some(name) = stream.next_name().context(errors::INTAKE_STORAGE)? {
            entries = entries.checked_add(1).context(errors::INTAKE_STORAGE)?;
            ensure!(
                entries <= super::intake_constants::MAX_DIRECTORY_ENTRIES,
                errors::INTAKE_STORAGE
            );
            if name
                .to_str()
                .is_some_and(super::intake_record::is_pending_file)
            {
                occurrences = occurrences.checked_add(1).context(errors::INTAKE_STORAGE)?;
                ensure!(
                    occurrences <= super::intake_constants::MAX_PENDING_OCCURRENCES,
                    errors::INTAKE_STORAGE
                );
            }
        }
        Ok(IntakeCapacity {
            entries,
            occurrences,
        })
    }
}

fn verify_limit(observed: usize, limit: usize, staged: bool) -> Result<()> {
    let available = if staged {
        observed <= limit
    } else {
        observed < limit
    };
    ensure!(available, errors::INTAKE_STORAGE);
    Ok(())
}
