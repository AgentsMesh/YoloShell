use crate::{LocalTabView, TabCatalog, TabEntry, catalog::validate_name, constants::*};
use anyhow::{Result, ensure};
use uuid::Uuid;

impl TabCatalog {
    pub fn create(&mut self, name: &str) -> Result<Uuid> {
        ensure!(self.entries().len() < MAX_TABS, COUNT_ERROR);
        validate_name(name)?;
        ensure!(
            !self.entries().iter().any(|entry| entry.name == name),
            NAME_CONFLICT
        );
        let id = Uuid::new_v4();
        self.0.entries.push(TabEntry {
            id,
            name: name.into(),
            view: LocalTabView::default(),
        });
        self.0.active = id;
        Ok(id)
    }

    pub fn rename(&mut self, id: Uuid, name: &str) -> Result<()> {
        validate_name(name)?;
        ensure!(
            !self
                .entries()
                .iter()
                .any(|entry| entry.id != id && entry.name == name),
            NAME_CONFLICT
        );
        let entry = self
            .0
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or_else(|| anyhow::anyhow!(ACTIVE_ERROR))?;
        entry.name = name.into();
        Ok(())
    }

    /// Replaces one tab's view without selecting it. Presentation reconciliation
    /// uses this to retire panes in tabs the user is not currently viewing.
    pub fn replace_view(&mut self, id: Uuid, view: LocalTabView) -> Result<()> {
        let entry = self
            .0
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or_else(|| anyhow::anyhow!(ACTIVE_ERROR))?;
        entry.view = view;
        Ok(())
    }

    /// Removing a view never asks the Hub to terminate any of its terminals.
    pub fn remove(&mut self, id: Uuid) -> Result<()> {
        ensure!(self.entries().len() > 1, LAST_TAB);
        let index = self
            .entries()
            .iter()
            .position(|entry| entry.id == id)
            .ok_or_else(|| anyhow::anyhow!(ACTIVE_ERROR))?;
        self.0.entries.remove(index);
        if self.0.active == id {
            self.0.active = self.entries()[index.min(self.entries().len() - 1)].id;
        }
        Ok(())
    }
}
