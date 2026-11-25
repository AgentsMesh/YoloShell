use super::*;

pub(crate) const CREATION_QUERY_SPEC_CONFLICT: &str =
    "creation token is bound to another Host request";
pub(crate) const CREATION_QUERY_RECEIPT_CONFLICT: &str =
    "creation query receipt identity is inconsistent";
pub(crate) const CREATION_QUERY_PHASE_CONFLICT: &str =
    "creation query completion conflicts with active phase";

impl CreationLedger {
    pub(in crate::host) fn query_creation(
        &self,
        token: Uuid,
        fingerprint: &str,
        receipts: &CreationReceiptStore,
    ) -> std::result::Result<crate::model::CreationQueryResult, ErrorBody> {
        use crate::model::{CreationQueryOutcome as Outcome, CreationQueryResult};
        self.ensure_healthy()?;
        let digest = creation_token_digest(token);
        let binding = self
            .bindings
            .iter()
            .find(|entry| entry.creation_token_digest == digest);
        if binding.is_some_and(|entry| entry.spec.request_fingerprint != fingerprint) {
            return Err(ErrorBody::conflict(CREATION_QUERY_SPEC_CONFLICT));
        }
        let by_token = receipts.lookup_token_digest(&digest).map_err(internal)?;
        let identity = binding
            .map(|entry| entry.result)
            .or_else(|| by_token.as_ref().map(|receipt| receipt.result));
        let receipt = match identity {
            Some(result) => match receipts
                .lookup_identity(&digest, result)
                .map_err(internal)?
            {
                CreationReceiptLookup::Matched(receipt) => {
                    if by_token.as_ref().is_some_and(|first| first != &receipt) {
                        return Err(ErrorBody::internal(CREATION_QUERY_RECEIPT_CONFLICT));
                    }
                    Some(receipt)
                }
                CreationReceiptLookup::Missing if by_token.is_none() => None,
                _ => return Err(ErrorBody::internal(CREATION_QUERY_RECEIPT_CONFLICT)),
            },
            None => None,
        };
        let outcome = match receipt {
            Some(receipt) => query_receipt_outcome(&receipt, binding, fingerprint)?,
            None => match binding {
                Some(binding) if binding.phase == CreationPhase::Ready => Outcome::Ready {
                    creation: binding.result,
                },
                Some(_) => Outcome::Pending {},
                None => Outcome::Missing {},
            },
        };
        Ok(CreationQueryResult {
            creation_token: token,
            outcome,
        })
    }
}

pub(crate) fn query_receipt_outcome(
    receipt: &CreationReceipt,
    binding: Option<&CreationBinding>,
    fingerprint: &str,
) -> std::result::Result<crate::model::CreationQueryOutcome, ErrorBody> {
    use crate::model::CreationQueryOutcome as Outcome;
    if receipt.request_fingerprint != fingerprint {
        return Err(ErrorBody::conflict(CREATION_QUERY_SPEC_CONFLICT));
    }
    if let Some(binding) = binding {
        let valid = match receipt.completion {
            CreationCompletion::Finalized | CreationCompletion::Survivor => {
                binding.phase == CreationPhase::Ready
            }
            CreationCompletion::Archived => binding.phase.permits_product_archive(),
        };
        if !valid {
            return Err(ErrorBody::internal(CREATION_QUERY_PHASE_CONFLICT));
        }
    }
    Ok(match receipt.completion {
        CreationCompletion::Finalized => Outcome::Finalized {
            creation: receipt.result,
        },
        CreationCompletion::Survivor => Outcome::Survivor {
            creation: receipt.result,
        },
        CreationCompletion::Archived => Outcome::Archived {
            terminal_id: receipt.result.terminal_id,
        },
    })
}
