use crate::{LocalTabView, TabCatalog, TabEntry, catalog::validate_name, constants::*};
use anyhow::{Result, ensure};
use uuid::Uuid;
use yolo_hub_identity::HubId;

impl TabCatalog {
    /// Switches to a machine workspace, creating one empty tab when it has none.
    pub fn select_machine(&mut self, machine: HubId) -> Result<bool> {
        if self.machine() == machine && self.active_by_machine().contains_key(&machine) {
            return Ok(false);
        }
        if let Some(id) = self.active_by_machine().get(&machine).copied()
            && self
                .entries()
                .iter()
                .any(|entry| entry.id == id && entry.machine == machine)
        {
            self.0.active = id;
            self.0.machine = machine;
            return Ok(true);
        }
        let name = self.next_name(machine)?;
        let id = Uuid::new_v4();
        self.0.entries.push(TabEntry {
            id,
            name,
            view: LocalTabView::default(),
            machine,
        });
        self.0.active_by_machine.insert(machine, id);
        self.0.active = id;
        self.0.machine = machine;
        Ok(true)
    }

    pub fn create(&mut self, name: &str) -> Result<Uuid> {
        let machine = self.machine();
        ensure!(self.count_for(machine) < MAX_TABS, COUNT_ERROR);
        validate_name(name)?;
        ensure!(
            !self
                .entries()
                .iter()
                .any(|entry| entry.machine == machine && entry.name == name),
            NAME_CONFLICT
        );
        let id = Uuid::new_v4();
        self.0.entries.push(TabEntry {
            id,
            name: name.into(),
            view: LocalTabView::default(),
            machine,
        });
        self.0.active = id;
        self.0.active_by_machine.insert(machine, id);
        Ok(id)
    }

    pub fn rename(&mut self, id: Uuid, name: &str) -> Result<()> {
        validate_name(name)?;
        let machine = self
            .entries()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.machine)
            .ok_or_else(|| anyhow::anyhow!(ACTIVE_ERROR))?;
        ensure!(
            !self
                .entries()
                .iter()
                .any(|entry| entry.id != id && entry.machine == machine && entry.name == name),
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
        let machine = self
            .entries()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.machine)
            .ok_or_else(|| anyhow::anyhow!(ACTIVE_ERROR))?;
        ensure!(self.count_for(machine) > 1, LAST_TAB);
        self.remove_entry(id)
    }

    /// Low-level removal used after a dead terminal empties a tab. The owning
    /// machine must still have another tab, or the caller must replace the view.
    pub fn remove_entry(&mut self, id: Uuid) -> Result<()> {
        let index = self
            .entries()
            .iter()
            .position(|entry| entry.id == id)
            .ok_or_else(|| anyhow::anyhow!(ACTIVE_ERROR))?;
        let machine = self.entries()[index].machine;
        let before = self
            .entries()
            .iter()
            .filter(|entry| entry.machine == machine)
            .position(|entry| entry.id == id)
            .unwrap_or_default();
        self.0.entries.remove(index);
        self.0.documents.remove(&id);
        let remaining: Vec<Uuid> = self
            .entries()
            .iter()
            .filter(|entry| entry.machine == machine)
            .map(|entry| entry.id)
            .collect();
        if remaining.is_empty() {
            self.0.active_by_machine.remove(&machine);
            ensure!(!self.0.entries.is_empty(), LAST_TAB);
            if self.0.machine == machine {
                self.0.machine = self.0.entries[0].machine;
            }
        } else {
            let next = remaining[before.min(remaining.len() - 1)];
            if self.0.active_by_machine.get(&machine) == Some(&id) {
                self.0.active_by_machine.insert(machine, next);
            }
            if self.0.active == id {
                self.0.active = next;
            }
        }
        let active_machine = self.0.machine;
        if self.0.active_by_machine.get(&active_machine) != Some(&self.0.active) {
            self.0.active = self.0.active_by_machine[&active_machine];
        }
        Ok(())
    }

    /// First unused `Tab N` for a machine.
    pub fn next_name(&self, machine: HubId) -> Result<String> {
        ensure!(self.count_for(machine) < MAX_TABS, COUNT_ERROR);
        (1..=MAX_TABS)
            .map(|number| format!("{TAB_NAME_PREFIX} {number}"))
            .find(|name| {
                !self
                    .entries()
                    .iter()
                    .any(|entry| entry.machine == machine && entry.name == *name)
            })
            .ok_or_else(|| anyhow::anyhow!(COUNT_ERROR))
    }

    pub fn count_for(&self, machine: HubId) -> usize {
        self.entries()
            .iter()
            .filter(|entry| entry.machine == machine)
            .count()
    }
}
