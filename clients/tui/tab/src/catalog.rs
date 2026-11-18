use crate::{DocumentPane, LocalTabView, TabViewMode, constants::*};
use std::collections::BTreeMap;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabEntry {
    pub id: Uuid,
    pub name: String,
    pub view: LocalTabView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabCatalogDocument {
    pub active: Uuid,
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
    pub fn single(view: LocalTabView) -> Self {
        let id = Uuid::new_v4();
        Self(TabCatalogDocument {
            active: id,
            entries: vec![TabEntry {
                id,
                name: DEFAULT_TAB_NAME.into(),
                view,
            }],
            documents: Default::default(),
        })
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

    pub fn update_active(&mut self, view: LocalTabView) {
        self.0
            .entries
            .iter_mut()
            .find(|entry| entry.id == self.0.active)
            .expect(ACTIVE_ERROR)
            .view = view;
    }

    pub fn active_layout(&self) -> &yolo_presentation_layout::Layout {
        &self.active().view.layout
    }

    pub fn active_layout_mut(&mut self) -> &mut yolo_presentation_layout::Layout {
        &mut self
            .0
            .entries
            .iter_mut()
            .find(|entry| entry.id == self.0.active)
            .expect(ACTIVE_ERROR)
            .view
            .layout
    }

    pub fn replace_active_layout(&mut self, layout: yolo_presentation_layout::Layout) {
        self.active_layout_mut().clone_from(&layout);
    }

    pub fn active_view(&self) -> TabViewMode {
        self.active().view.view
    }

    /// Document panes are client-local and keyed by their owning tab. They are
    /// not part of the shared layout, so publishing or adopting a layout never
    /// changes the terminal topology.
    pub fn active_document(&self) -> Option<&DocumentPane> {
        self.0.documents.get(&self.0.active)
    }

    pub fn open_document(&mut self, title: String, text: String) -> bool {
        self.0
            .documents
            .insert(self.0.active, DocumentPane { title, text });
        true
    }

    pub fn close_document(&mut self) -> bool {
        self.0.documents.remove(&self.0.active).is_some()
    }

    pub fn replace_active_view(&mut self, view: TabViewMode) {
        self.0
            .entries
            .iter_mut()
            .find(|entry| entry.id == self.0.active)
            .expect(ACTIVE_ERROR)
            .view
            .view = view;
    }

    pub fn select(&mut self, id: Uuid) -> Result<bool> {
        ensure!(
            self.entries().iter().any(|entry| entry.id == id),
            ACTIVE_ERROR
        );
        let changed = self.0.active != id;
        self.0.active = id;
        Ok(changed)
    }

    pub fn cycle(&mut self, forward: bool) {
        let index = self
            .entries()
            .iter()
            .position(|entry| entry.id == self.0.active)
            .expect(ACTIVE_ERROR);
        let count = self.entries().len();
        let next = if forward {
            (index + 1) % count
        } else {
            (index + count - 1) % count
        };
        self.0.active = self.entries()[next].id;
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
        ensure!(
            !document.entries.is_empty() && document.entries.len() <= MAX_TABS,
            COUNT_ERROR
        );
        let mut ids = BTreeSet::new();
        for entry in &document.entries {
            ensure!(!entry.id.is_nil() && ids.insert(entry.id), ID_ERROR);
            validate_name(&entry.name)?;
        }
        ensure!(ids.contains(&document.active), ACTIVE_ERROR);
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
