use crate::{CreationIntent, CreationJournal, constants::*};
use anyhow::{Result, ensure};

impl CreationJournal {
    pub fn intents(&self) -> Result<Vec<CreationIntent>> {
        self.verify()?;
        Ok(self.document.intents.clone())
    }

    /// Returns true only after newly persisting this intent, before initial Create.
    /// False means an exact intent already exists; it MUST NOT authorize resending Create.
    pub fn retain_intent(&mut self, intent: CreationIntent) -> Result<bool> {
        let result = (|| {
            self.check_current()?;
            intent.validate()?;
            ensure!(
                !self
                    .document
                    .pending
                    .iter()
                    .any(|record| intent.overlaps_request(&record.request)),
                CONFLICT
            );
            if let Some(existing) = self
                .document
                .intents
                .iter()
                .find(|other| intent.overlaps_request(&other.request))
            {
                ensure!(*existing == intent, CONFLICT);
                return Ok(false);
            }
            ensure!(
                self.document.pending.len() + self.document.intents.len() < MAX_PENDING_CREATIONS,
                CAPACITY
            );
            let mut next = self.document.clone();
            next.intents.push(intent);
            self.install(next)?;
            Ok(true)
        })();
        self.retain_failure(result)
    }

    /// Remove only after an authenticated origin query for this exact original
    /// request proves Finalized, Survivor, or Archived. Missing, Pending, Unknown,
    /// transport failures, and uncorrelated replies MUST retain the intent.
    /// This method sends no request and confers no permission to retry Create.
    pub fn settle_intent(&mut self, exact: &CreationIntent) -> Result<bool> {
        self.remove_intent_exact(exact)
    }

    /// Cancel only with live proof that this exact initial attempt created no resource:
    /// local non-dispatch, or an authenticated origin rejection before creation admission.
    /// Generic remote errors, unknown delivery, a Missing query and restarted records
    /// are not such proof. This is never permission to replay an uncertain Create.
    pub fn cancel_uncreated_intent(&mut self, exact: &CreationIntent) -> Result<bool> {
        self.remove_intent_exact(exact)
    }

    fn remove_intent_exact(&mut self, exact: &CreationIntent) -> Result<bool> {
        let result = (|| {
            self.check_current()?;
            exact.validate()?;
            let Some(index) = self
                .document
                .intents
                .iter()
                .position(|other| exact.overlaps_request(&other.request))
            else {
                return Ok(false);
            };
            ensure!(self.document.intents[index] == *exact, CONFLICT);
            let mut next = self.document.clone();
            next.intents.remove(index);
            self.install(next)?;
            Ok(true)
        })();
        self.retain_failure(result)
    }
}
