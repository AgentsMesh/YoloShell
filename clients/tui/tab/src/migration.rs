use crate::{
    DocumentPane, LayoutCheckpoint, LocalTabView, TabCatalog, TabCatalogDocument, TabDocument,
    TabEntry, constants::*,
};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_presentation_protocol::LayoutAddress;

pub(crate) const LEGACY_SCHEMA_VERSION: u16 = 5;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTabDocument {
    schema_version: u16,
    entry_hub: HubId,
    layout_address: LayoutAddress,
    shared: Option<LayoutCheckpoint>,
    revision: u64,
    tabs: LegacyTabCatalog,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTabCatalog {
    active: Uuid,
    entries: Vec<LegacyTabEntry>,
    #[serde(default)]
    documents: BTreeMap<Uuid, DocumentPane>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTabEntry {
    id: Uuid,
    name: String,
    view: LocalTabView,
}

pub(crate) fn schema_version(value: &Value) -> Option<u16> {
    u16::try_from(value.get(SCHEMA_VERSION_FIELD)?.as_u64()?).ok()
}

/// Assigns every v5 tab to a machine and rebuilds per-machine active maps.
/// Tabs in the last shared projection belong to the entry Hub; otherwise the
/// first bound terminal names the machine, falling back to the entry Hub.
pub(crate) fn migrate_v5(value: Value, entry: HubId) -> Result<TabDocument> {
    let legacy: LegacyTabDocument = serde_json::from_value(value)?;
    ensure!(legacy.schema_version == LEGACY_SCHEMA_VERSION, SCHEMA_ERROR);
    ensure!(legacy.entry_hub == entry, ENTRY_ERROR);
    let shared_ids = legacy
        .shared
        .iter()
        .flat_map(|shared| shared.projected.iter().map(|tab| tab.id))
        .collect::<BTreeSet<_>>();
    let legacy_active = legacy.tabs.active;
    let mut active_by_machine = BTreeMap::new();
    let mut active_machine = None;
    let mut entries = Vec::new();
    for tab in legacy.tabs.entries {
        let machine = if shared_ids.contains(&tab.id) {
            entry
        } else {
            tab.view
                .layout
                .panes()
                .into_iter()
                .find_map(|pane| pane.terminal.map(|address| address.hub_id))
                .unwrap_or(entry)
        };
        if tab.id == legacy_active {
            active_machine = Some(machine);
        }
        active_by_machine.entry(machine).or_insert(tab.id);
        entries.push(TabEntry {
            id: tab.id,
            name: tab.name,
            view: tab.view,
            machine,
        });
    }
    let machine = active_machine.unwrap_or(entry);
    let active = if entries
        .iter()
        .any(|entry| entry.id == legacy_active && entry.machine == machine)
    {
        legacy_active
    } else {
        *active_by_machine.get(&machine).ok_or_else(|| anyhow::anyhow!(ACTIVE_ERROR))?
    };
    active_by_machine.insert(machine, active);
    let tabs = TabCatalog::try_from(TabCatalogDocument {
        active,
        machine,
        active_by_machine,
        entries,
        documents: legacy.tabs.documents,
    })?;
    Ok(TabDocument {
        schema_version: TAB_SCHEMA_VERSION,
        entry_hub: entry,
        layout_address: legacy.layout_address,
        shared: legacy.shared,
        revision: legacy.revision,
        tabs,
    })
}
