use super::super::CodexHookArtifacts;
use anyhow::{Context, Result};

const RECOVERY_CORRUPT: &str = "Codex pre-admission staging recovery is corrupt";

pub(super) struct WriterGuard {
    entry: crate::paths::private_file::PrivateFileEntry,
    lock: crate::paths::private_file_lock::PrivateFileLock,
    identity: crate::paths::identity::FileIdentity,
}

pub(super) fn reclaim(
    invocation: &crate::paths::directory::PrivateDirectory,
    artifacts: &CodexHookArtifacts,
    admission: &crate::paths::private_file::PrivateFileEntry,
    staging: &crate::paths::private_file::PrivateFileEntry,
    residue: super::StagingResidue,
    writer_identity: crate::paths::identity::FileIdentity,
) -> Result<WriterGuard> {
    invocation.verify_path().context(RECOVERY_CORRUPT)?;
    let writer_entry = crate::paths::private_file::PrivateFileEntry::from_directory(
        invocation.try_clone()?,
        &artifacts.retirement_writer_lock_path(),
    )?;
    let writer_lock = crate::paths::private_file_lock::PrivateFileLock::try_acquire_existing_in(
        invocation.try_clone()?,
        writer_entry.path(),
        super::super::constants::LOCK_RECORD_MAX_BYTES,
    )?
    .context(RECOVERY_CORRUPT)?;
    let writer = WriterGuard {
        entry: writer_entry,
        lock: writer_lock,
        identity: writer_identity,
    };
    writer.verify()?;
    admission.verify_expected(None, crate::paths::MAX_DURABLE_JSON_BYTES)?;
    let residue_entry = match residue {
        super::StagingResidue::Active => staging.reopen()?,
        super::StagingResidue::Isolated => staging.unlink_isolation_entry()?,
    };
    let mut staging_lock =
        crate::paths::private_file_lock::PrivateFileLock::try_acquire_existing_in(
            invocation.try_clone()?,
            residue_entry.path(),
            crate::paths::MAX_DURABLE_JSON_BYTES,
        )?
        .context(RECOVERY_CORRUPT)?;
    writer.verify()?;
    admission.verify_expected(None, crate::paths::MAX_DURABLE_JSON_BYTES)?;
    staging_lock.verify()?;
    run_before_reclaim_hook();
    writer.verify()?;
    admission.verify_expected(None, crate::paths::MAX_DURABLE_JSON_BYTES)?;
    staging_lock.verify()?;
    match residue {
        super::StagingResidue::Active => staging_lock.reclaim_owned()?,
        super::StagingResidue::Isolated => staging_lock.reclaim_isolation_of(staging)?,
    }
    admission.verify_expected(None, crate::paths::MAX_DURABLE_JSON_BYTES)?;
    writer.verify()?;
    invocation.verify_path().context(RECOVERY_CORRUPT)?;
    Ok(writer)
}

impl WriterGuard {
    pub(super) fn verify(&self) -> Result<()> {
        self.lock.verify().context(RECOVERY_CORRUPT)?;
        self.entry
            .verify_current_unchanged(
                self.identity,
                super::super::constants::LOCK_RECORD_MAX_BYTES,
            )
            .context(RECOVERY_CORRUPT)?;
        self.lock.verify().context(RECOVERY_CORRUPT)
    }
}

#[cfg(test)]
thread_local! {
    static BEFORE_RECLAIM: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn set_before_reclaim_hook(hook: impl FnOnce() + 'static) {
    BEFORE_RECLAIM.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

fn run_before_reclaim_hook() {
    #[cfg(test)]
    BEFORE_RECLAIM.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}
