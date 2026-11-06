use crate::constants::LOST;
use anyhow::{Result, ensure};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// One-way revocation of a cache generation, independent of its filesystem I/O.
/// A commit already past its fence may settle; the old writer keeps its lock
/// until that I/O returns. A successor can never write concurrently with it.
#[derive(Clone, Default)]
pub struct Retirement(Arc<AtomicBool>);

impl Retirement {
    pub fn retire(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_retired(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(!self.is_retired(), LOST);
        Ok(())
    }
}
