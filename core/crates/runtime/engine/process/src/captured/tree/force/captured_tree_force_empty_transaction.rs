use super::{FrozenGoneAuthority, ProcessClosureVersion};
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct ProducersGoneTransaction(Arc<ProducersGoneTransactionAllocation>);

struct ProducersGoneTransactionAllocation {
    _private: (),
}

pub(super) enum ProducersGoneTransactionState {
    Prepared(ProducersGoneTransaction),
    CommitPrepared(ProducersGoneTransaction),
    Finalized(ProducersGoneTransaction),
}

pub struct FinalizedProducersGoneReady {
    pub(super) version: ProcessClosureVersion,
    pub(super) authority: FrozenGoneAuthority,
    pub(super) transaction: ProducersGoneTransaction,
}

pub enum FrozenProducersGoneFinalization {
    Finalized(FinalizedProducersGoneReady),
    OutputInvalidated,
    Refreeze,
}

impl ProducersGoneTransaction {
    pub(super) fn fresh() -> Self {
        Self(Arc::new(ProducersGoneTransactionAllocation {
            _private: (),
        }))
    }

    fn same_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl ProducersGoneTransactionState {
    pub(super) fn prepared(transaction: ProducersGoneTransaction) -> Self {
        Self::Prepared(transaction)
    }

    pub(super) fn matches_unfinalized(&self, expected: &ProducersGoneTransaction) -> bool {
        match self {
            Self::Prepared(current) | Self::CommitPrepared(current) => {
                current.same_allocation(expected)
            }
            Self::Finalized(_) => false,
        }
    }

    pub(super) fn prepare_commit(&mut self, expected: &ProducersGoneTransaction) -> bool {
        match self {
            Self::Prepared(current) if current.same_allocation(expected) => {
                *self = Self::CommitPrepared(current.clone());
                true
            }
            Self::CommitPrepared(current) => current.same_allocation(expected),
            _ => false,
        }
    }

    pub(super) fn matches_commit_prepared(&self, expected: &ProducersGoneTransaction) -> bool {
        matches!(self, Self::CommitPrepared(current) if current.same_allocation(expected))
    }

    pub(super) fn finalize(&mut self, expected: &ProducersGoneTransaction) -> bool {
        let Self::CommitPrepared(current) = self else {
            return false;
        };
        if !current.same_allocation(expected) {
            return false;
        }
        *self = Self::Finalized(current.clone());
        true
    }

    pub(super) fn matches_finalized(&self, expected: &ProducersGoneTransaction) -> bool {
        matches!(self, Self::Finalized(current) if current.same_allocation(expected))
    }
}
