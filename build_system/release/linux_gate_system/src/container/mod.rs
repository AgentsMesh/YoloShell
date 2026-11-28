mod cid;
pub(super) mod engine;
pub(super) mod model;

use anyhow::{Result, ensure};
use engine::{Engine, Selector};
use model::Contract;
use std::time::Instant;
use yolo_release_contracts::*;
use yolo_release_filesystem::{DirectoryHandle, DirectoryResourceUse};
use yolo_release_gate_system::GateSnapshot;

pub(super) struct Owner<'a> {
    snapshot: &'a GateSnapshot,
    engine: &'a dyn Engine,
    engine_identity: String,
    contract: Contract,
    custody: Option<DirectoryResourceUse>,
    cleanup_attempted: bool,
}
impl<'a> Owner<'a> {
    pub fn prepare(
        snapshot: &'a GateSnapshot,
        engine: &'a dyn Engine,
        contract: Contract,
        mounted_tool: Option<&DirectoryHandle>,
    ) -> Result<Self> {
        let engine_identity = engine.identity()?;
        match std::fs::symlink_metadata(&contract.command.cid_file) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
            Ok(_) => anyhow::bail!(CONTAINER_COLLISION),
        }
        ensure!(
            engine
                .find(Selector::Name(&contract.command.name))?
                .is_none(),
            CONTAINER_COLLISION
        );
        ensure!(engine.identity()? == engine_identity, ENGINE_CHANGED);
        let description = format!(
            "Docker container {} engine {}",
            contract.command.name, engine_identity
        );
        let custody = DirectoryResourceUse::retain(
            std::iter::once(snapshot.source()).chain(mounted_tool),
            &description,
        )?;
        Ok(Self {
            snapshot,
            engine,
            engine_identity,
            contract,
            custody: Some(custody),
            cleanup_attempted: false,
        })
    }

    pub fn finish(mut self, outcome: Result<()>) -> Result<()> {
        self.cleanup_attempted = true;
        let cleanup = self.settle(outcome.is_ok());
        match (outcome, cleanup) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(error),
            (result, Err(cleanup)) => Err(anyhow::anyhow!(
                "{}; {CONTAINER_UNCONFIRMED}: {cleanup:#}; retained {} container {}",
                result
                    .err()
                    .map_or_else(String::new, |error| format!("{error:#}")),
                self.snapshot.root_path().display(),
                self.contract.command.name
            )),
        }
    }

    fn verify_engine(&self) -> Result<()> {
        ensure!(
            self.engine.identity()? == self.engine_identity,
            ENGINE_CHANGED
        );
        Ok(())
    }

    fn settle(&mut self, successful_run: bool) -> Result<()> {
        self.verify_engine()?;
        let cid = cid::read(self.snapshot.temporary(), &self.contract.command.cid_file)?;
        let current = self.engine.find(match &cid {
            Some(id) => Selector::Id(id),
            None => Selector::Name(&self.contract.command.name),
        })?;
        self.verify_engine()?;
        let Some(current) = current else {
            ensure!(cid.is_some() || successful_run, CONTAINER_UNCONFIRMED);
            return self.confirm();
        };
        self.contract.verify(&current)?;
        self.verify_engine()?;
        let removal = self.engine.remove(&current.id);
        let deadline = Instant::now() + CONTAINER_SETTLEMENT_TIMEOUT;
        loop {
            self.verify_engine()?;
            match self.engine.find(Selector::Id(&current.id))? {
                None => {
                    self.verify_engine()?;
                    return self.confirm();
                }
                Some(observed) => self.contract.verify(&observed)?,
            }
            self.verify_engine()?;
            ensure!(
                Instant::now() < deadline,
                "{CONTAINER_UNCONFIRMED}: {removal:?}"
            );
            std::thread::sleep(CONTAINER_SETTLEMENT_POLL);
        }
    }

    fn confirm(&mut self) -> Result<()> {
        self.custody
            .take()
            .expect(CONTAINER_UNCONFIRMED)
            .confirm_released();
        Ok(())
    }
}
impl Drop for Owner<'_> {
    fn drop(&mut self) {
        if !self.cleanup_attempted {
            self.cleanup_attempted = true;
            if let Err(error) = self.settle(false) {
                eprintln!(
                    "{CONTAINER_UNCONFIRMED}: {error:#}; retained {} container {}",
                    self.snapshot.root_path().display(),
                    self.contract.command.name
                );
            }
        }
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/container_owner.rs"]
mod tests;
