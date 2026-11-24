use crate::{
    DEFAULT_TAB_NAME, LocalTabView, TabCatalog, TabCatalogDocument, TabEntry, TabViewMode,
    constants::*,
};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_presentation_layout::{Layout, PaneLayoutDocument as PaneView};
use yolo_presentation_protocol::{SharedLayoutDocument, SharedTab};

impl TabCatalog {
    /// Export one machine's topology and bindings. Remote machine workspaces
    /// are client-local and never enter the entry Hub's shared layout.
    pub fn shared_tabs(&self, machine: HubId) -> Result<Vec<SharedTab>> {
        let entries = self
            .entries()
            .iter()
            .filter(|entry| entry.machine == machine)
            .collect::<Vec<_>>();
        ensure!(!entries.is_empty(), SHARED_TABS_EMPTY);
        entries
            .into_iter()
            .map(|entry| {
                Ok(SharedTab::new(
                    entry.id,
                    entry.name.clone(),
                    entry.view.layout.document().root.clone(),
                )?)
            })
            .collect()
    }

    /// Reconcile the entry machine's shared document while preserving every
    /// other machine workspace and the client-local selection.
    pub fn from_shared(
        document: &SharedLayoutDocument,
        previous: &Self,
        entry_hub: HubId,
    ) -> Result<Self> {
        document.validate()?;
        let old_active = previous.active_by_machine().get(&entry_hub).copied();
        let mut shared = Vec::new();
        if document.tabs.is_empty() {
            let id = previous
                .entries()
                .iter()
                .find(|entry| entry.machine == entry_hub)
                .map(|entry| entry.id)
                .unwrap_or_else(Uuid::new_v4);
            shared.push(TabEntry {
                id,
                name: DEFAULT_TAB_NAME.into(),
                view: LocalTabView::default(),
                machine: entry_hub,
            });
        } else {
            for tab in &document.tabs {
                let old = previous
                    .entries()
                    .iter()
                    .find(|entry| entry.id == tab.id && entry.machine == entry_hub);
                let panes = tab.root.panes();
                let focused = old
                    .map(|entry| entry.view.layout.focused())
                    .filter(|id| panes.iter().any(|pane| pane.id == *id))
                    .unwrap_or(panes[0].id);
                shared.push(TabEntry {
                    id: tab.id,
                    name: tab.title.clone(),
                    view: LocalTabView {
                        layout: Layout::try_from(PaneView {
                            root: tab.root.clone(),
                            focused,
                        })?,
                        view: old.map_or(TabViewMode::Terminal, |entry| entry.view.view),
                    },
                    machine: entry_hub,
                });
            }
        }
        let incoming = shared.iter().map(|entry| entry.id).collect::<BTreeSet<_>>();
        let mut entries = shared;
        entries.extend(
            previous
                .entries()
                .iter()
                .filter(|entry| entry.machine != entry_hub && !incoming.contains(&entry.id))
                .cloned(),
        );
        let mut active_by_machine = BTreeMap::new();
        for (machine, id) in previous.active_by_machine() {
            if *machine == entry_hub {
                continue;
            }
            let chosen = entries
                .iter()
                .find(|entry| entry.machine == *machine && entry.id == *id)
                .map(|entry| entry.id)
                .or_else(|| {
                    entries
                        .iter()
                        .find(|entry| entry.machine == *machine)
                        .map(|entry| entry.id)
                });
            if let Some(chosen) = chosen {
                active_by_machine.insert(*machine, chosen);
            }
        }
        let entry_active = entries
            .iter()
            .find(|entry| entry.machine == entry_hub && Some(entry.id) == old_active)
            .map(|entry| entry.id)
            .unwrap_or(entries[0].id);
        active_by_machine.insert(entry_hub, entry_active);
        for entry in &entries {
            active_by_machine.entry(entry.machine).or_insert(entry.id);
        }
        let machine = if entries
            .iter()
            .any(|entry| entry.machine == previous.machine())
        {
            previous.machine()
        } else {
            entry_hub
        };
        let active = active_by_machine[&machine];
        let ids = entries.iter().map(|entry| entry.id).collect::<BTreeSet<_>>();
        let documents = previous
            .0
            .documents
            .iter()
            .filter(|(id, _)| ids.contains(id))
            .map(|(id, document)| (*id, document.clone()))
            .collect();
        Self::try_from(TabCatalogDocument {
            active,
            machine,
            active_by_machine,
            entries,
            documents,
        })
    }
}
