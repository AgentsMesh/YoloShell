use super::*;

pub(crate) const CREATION_QUERY_RETAINED_ENTRY_MISSING: &str =
    "creation query has no retained ledger entry";
pub(crate) const CREATION_QUERY_LEDGER_CHANGED: &str =
    "creation query durable ledger differs from retained state";
pub(crate) const CREATION_QUERY_NIL_TOKEN: &str = "creation query token must not be nil";

impl Host {
    pub(in crate::host) async fn query_creation(
        &self,
        request: CreateTerminal,
    ) -> std::result::Result<crate::model::CreationQueryResult, ErrorBody> {
        if request.creation_token.is_nil() {
            return Err(ErrorBody::invalid(CREATION_QUERY_NIL_TOKEN));
        }
        let fingerprint = CreationSpec::fingerprint(&request)?;
        let deadline = HostOperationDeadline::after(self.worker_ready_timeout);
        let ledger = self.lock_creation_capabilities_before(deadline).await?;
        ledger.verify_query_snapshot()?;
        let result = ledger.query_creation(
            request.creation_token,
            &fingerprint,
            &self.creation_receipts,
        )?;
        require_creation_budget(deadline)?;
        Ok(result)
    }
}

impl CreationLedger {
    pub(crate) fn verify_query_snapshot(&self) -> std::result::Result<(), ErrorBody> {
        self.ensure_healthy()?;
        self.validate().map_err(internal)?;
        let entry = self
            .entry
            .as_ref()
            .ok_or_else(|| ErrorBody::internal(CREATION_QUERY_RETAINED_ENTRY_MISSING))?;
        let persisted: Option<Self> = read_creation_ledger_entry(entry).map_err(internal)?;
        if let Some(persisted) = &persisted {
            persisted.validate().map_err(internal)?;
        }
        let expected = persisted.unwrap_or_default();
        if self != &expected {
            return Err(ErrorBody::internal(CREATION_QUERY_LEDGER_CHANGED));
        }
        Ok(())
    }
}
