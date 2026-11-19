use crate::{TabCatalogDocument, TabEntry, TabViewMode, LocalTabView, TabCatalog};
use anyhow::Result;
use std::collections::BTreeSet;
use yolo_presentation_layout::{Layout, PaneLayoutDocument as PaneView};
use yolo_presentation_protocol::{SharedLayoutDocument, SharedTab};

impl TabCatalog {
    /// Export topology and terminal bindings without publishing this client's focus or view mode.
    pub fn shared_tabs(&self) -> Result<Vec<SharedTab>> {
        self.entries()
            .iter()
            .map(|entry| {
                Ok(SharedTab::new(
                    entry.id,
                    entry.name.clone(),
                    entry.view.layout.document().root.clone(),
                )?)
            })
            .collect()
    }

    /// Reconcile a shared document while retaining every surviving client-local selection.
    /// Empty documents show a local placeholder; they do not create a terminal or shared tab.
    pub fn from_shared(document: &SharedLayoutDocument, previous: &Self) -> Result<Self> {
        document.validate()?;
        if document.tabs.is_empty() {
            return Ok(Self::default());
        }
        let entries = document
            .tabs
            .iter()
            .map(|tab| {
                let old = previous.entries().iter().find(|entry| entry.id == tab.id);
                let panes = tab.root.panes();
                let focused = old
                    .map(|entry| entry.view.layout.focused())
                    .filter(|id| panes.iter().any(|pane| pane.id == *id))
                    .unwrap_or(panes[0].id);
                Ok(TabEntry {
                    id: tab.id,
                    name: tab.title.clone(),
                    view: LocalTabView {
                        layout: Layout::try_from(PaneView {
                            root: tab.root.clone(),
                            focused,
                        })?,
                        view: old.map_or(TabViewMode::Terminal, |entry| entry.view.view),
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let active = previous.active().id;
        let active = entries
            .iter()
            .find(|entry| entry.id == active)
            .unwrap_or(&entries[0])
            .id;
        let ids = entries
            .iter()
            .map(|entry| entry.id)
            .collect::<BTreeSet<_>>();
        let documents = previous
            .0
            .documents
            .iter()
            .filter(|(id, _)| ids.contains(id))
            .map(|(id, document)| (*id, document.clone()))
            .collect();
        Self::try_from(TabCatalogDocument {
            active,
            entries,
            documents,
        })
    }
}
