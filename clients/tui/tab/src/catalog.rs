use crate::{DocumentPane, LocalTabView, constants::*};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
use yolo_hub_identity::HubId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabEntry {
    pub id: Uuid,
    pub name: String,
    pub view: LocalTabView,
    /// The machine whose workspace owns this tab. Tabs never switch machines.
    pub machine: HubId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabCatalogDocument {
    /// Active tab of `machine`.
    pub active: Uuid,
    /// The machine workspace currently shown.
    pub machine: HubId,
    /// Active tab per machine, including `machine`.
    pub active_by_machine: BTreeMap<HubId, Uuid>,
    pub entries: Vec<TabEntry>,
    /// Plugin documents are client-local presentation content. They persist
    /// with this tab document but never enter the shared terminal layout.
    /// `default` keeps older files that predate documents decodable.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub documents: BTreeMap<Uuid, DocumentPane>,
}

/// The runtime tab catalog and its persisted representation.
///
/// The aggregate owns active layout and selection. Persistence is deliberately
/// converted through `TabCatalogDocument` so storage details do not leak into the
/// TUI state model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TabCatalogDocument", into = "TabCatalogDocument")]
pub struct TabCatalog(pub(crate) TabCatalogDocument);

impl TabCatalog {
    pub fn new(machine: HubId, view: LocalTabView) -> Self {
        let id = Uuid::new_v4();
        let mut active_by_machine = BTreeMap::new();
        active_by_machine.insert(machine, id);
        Self(TabCatalogDocument {
            active: id,
            machine,
            active_by_machine,
            entries: vec![TabEntry {
                id,
                name: DEFAULT_TAB_NAME.into(),
                view,
                machine,
            }],
            documents: Default::default(),
        })
    }

    pub fn single(view: LocalTabView) -> Self {
        Self::new(HubId::new(), view)
    }

    pub fn machine(&self) -> HubId {
        self.0.machine
    }

    pub fn active_by_machine(&self) -> &BTreeMap<HubId, Uuid> {
        &self.0.active_by_machine
    }

    pub fn machine_entries(&self) -> Vec<&TabEntry> {
        self.entries()
            .iter()
            .filter(|entry| entry.machine == self.0.machine)
            .collect()
    }

    pub fn active(&self) -> &TabEntry {
        self.entries()
            .iter()
            .find(|entry| entry.id == self.0.active)
            .expect(ACTIVE_ERROR)
    }

    pub fn entries(&self) -> &[TabEntry] {
        &self.0.entries
    }

    pub fn select(&mut self, id: Uuid) -> Result<bool> {
        let machine = self
            .entries()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.machine)
            .ok_or_else(|| anyhow::anyhow!(ACTIVE_ERROR))?;
        let changed = self.0.active != id || self.0.machine != machine;
        self.0.active = id;
        self.0.machine = machine;
        self.0.active_by_machine.insert(machine, id);
        Ok(changed)
    }

    pub fn cycle(&mut self, forward: bool) {
        let entries = self.machine_entries();
        let index = entries
            .iter()
            .position(|entry| entry.id == self.0.active)
            .expect(ACTIVE_ERROR);
        let count = entries.len();
        let next = if forward {
            (index + 1) % count
        } else {
            (index + count - 1) % count
        };
        self.0.active = entries[next].id;
        self.0
            .active_by_machine
            .insert(self.0.machine, self.0.active);
    }
}

impl Default for TabCatalog {
    fn default() -> Self {
        Self::single(LocalTabView::default())
    }
}

impl TryFrom<TabCatalogDocument> for TabCatalog {
    type Error = anyhow::Error;
    fn try_from(document: TabCatalogDocument) -> Result<Self> {
        ensure!(!document.entries.is_empty(), COUNT_ERROR);
        let mut ids = BTreeSet::new();
        let mut machines = BTreeMap::<HubId, usize>::new();
        for entry in &document.entries {
            ensure!(!entry.id.is_nil() && ids.insert(entry.id), ID_ERROR);
            validate_name(&entry.name)?;
            *machines.entry(entry.machine).or_default() += 1;
        }
        ensure!(
            machines.values().all(|count| *count <= MAX_TABS)
                && machines.len() == document.active_by_machine.len(),
            COUNT_ERROR
        );
        ensure!(ids.contains(&document.active), ACTIVE_ERROR);
        let active = document
            .entries
            .iter()
            .find(|entry| entry.id == document.active)
            .ok_or_else(|| anyhow::anyhow!(ACTIVE_ERROR))?;
        ensure!(active.machine == document.machine, ACTIVE_ERROR);
        for (machine, id) in &document.active_by_machine {
            ensure!(
                machines.contains_key(machine)
                    && document
                        .entries
                        .iter()
                        .any(|entry| entry.id == *id && entry.machine == *machine),
                ACTIVE_ERROR
            );
        }
        ensure!(
            document.active_by_machine.get(&document.machine) == Some(&document.active),
            ACTIVE_ERROR
        );
        Ok(Self(document))
    }
}

impl From<TabCatalog> for TabCatalogDocument {
    fn from(value: TabCatalog) -> Self {
        value.0
    }
}

pub(crate) fn validate_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty()
            && name.len() <= MAX_TAB_NAME_BYTES
            && name.trim() == name
            && !name.chars().any(char::is_control),
        NAME_ERROR
    );
    Ok(())
}
