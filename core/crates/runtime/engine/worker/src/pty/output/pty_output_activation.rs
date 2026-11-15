use super::{ExclusivePtyOutput, ReaderBackedPtyOutput};
use crate::worker::ReaderLifecycle;
use anyhow::{Error, Result};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub(crate) const READER_LIFECYCLE_ADOPTION_INVARIANT: &str =
    "prepared PTY output requires a fresh reader lifecycle";

#[cfg(test)]
pub(crate) struct PreparedPtyOutput {
    pub(crate) output: ExclusivePtyOutput,
}

impl ExclusivePtyOutput {
    pub(crate) fn prepare_reader_activation(&mut self) -> Result<()> {
        if let Some(journal) = self.startup_journal.as_mut() {
            journal.commit_pending()?;
        }
        Ok(())
    }

    pub(crate) fn complete_reader_activation(
        self,
        reader_lifecycle: Arc<ReaderLifecycle>,
    ) -> ReaderBackedPtyOutput {
        self.activate_reader(reader_lifecycle, None, None, || Ok(()))
            .unwrap_or_else(|_| unreachable!("{READER_LIFECYCLE_ADOPTION_INVARIANT}"))
    }

    pub(crate) fn activate_runtime_reader<Activate>(
        self,
        reader_lifecycle: Arc<ReaderLifecycle>,
        child_waited: Arc<AtomicBool>,
        runtime: Arc<crate::worker::pty_runtime_alias_custody::PtyRuntimeAliasCustody>,
        activate: Activate,
    ) -> std::result::Result<ReaderBackedPtyOutput, (Error, Self)>
    where
        Activate: FnOnce() -> Result<()>,
    {
        self.activate_reader(
            reader_lifecycle,
            Some(child_waited),
            Some(runtime),
            activate,
        )
    }

    pub(crate) fn activate_reader<Activate>(
        mut self,
        reader_lifecycle: Arc<ReaderLifecycle>,
        child_waited: Option<Arc<AtomicBool>>,
        runtime: Option<Arc<crate::worker::pty_runtime_alias_custody::PtyRuntimeAliasCustody>>,
        activate: Activate,
    ) -> std::result::Result<ReaderBackedPtyOutput, (Error, Self)>
    where
        Activate: FnOnce() -> Result<()>,
    {
        if let Err(error) = reader_lifecycle.adopt_exclusive_kernel_eof_before(&mut self, activate)
        {
            return Err((error, self));
        }
        drop(self.startup_journal);
        Ok(ReaderBackedPtyOutput {
            descriptor: Arc::new(super::SharedOutputDescriptor::new(self.descriptor, runtime)),
            reader_lifecycle: Some(reader_lifecycle),
            child_waited,
            master_alias_authority: self.master_alias_authority,
        })
    }

    #[cfg(test)]
    pub(crate) fn prepare_activation(mut self) -> Result<PreparedPtyOutput> {
        self.prepare_reader_activation()?;
        Ok(PreparedPtyOutput { output: self })
    }

    #[cfg(test)]
    pub(crate) fn into_reader_backed_for_test(self) -> ReaderBackedPtyOutput {
        self.prepare_activation()
            .unwrap_or_else(|_| unreachable!())
            .complete_for_test()
    }
}

#[cfg(test)]
impl PreparedPtyOutput {
    pub(crate) fn complete(self, reader_lifecycle: Arc<ReaderLifecycle>) -> ReaderBackedPtyOutput {
        self.output.complete_reader_activation(reader_lifecycle)
    }

    pub(crate) fn complete_for_test(self) -> ReaderBackedPtyOutput {
        drop(self.output.startup_journal);
        ReaderBackedPtyOutput {
            descriptor: Arc::new(super::SharedOutputDescriptor::new(
                self.output.descriptor,
                None,
            )),
            reader_lifecycle: None,
            child_waited: None,
            master_alias_authority: None,
        }
    }
}
