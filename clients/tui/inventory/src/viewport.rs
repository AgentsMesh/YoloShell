use crate::{
    Inventory, InventorySelection, View, constants::SELECTION_STEP, viewport_evidence::RowEvidence,
};
use ratatui::layout::{Position, Rect};
use std::collections::BTreeMap;
use yolo_hub_identity::{AgentTarget, HubId, TerminalAddress};
use yolo_hub_terminal_protocol::TerminalSummary;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventoryHit {
    Tab(View),
    Row(InventorySelection),
}

#[derive(Clone, Debug)]
pub struct InventoryViewport {
    area: Rect,
    rows_area: Rect,
    tabs: Vec<(Rect, View)>,
    rows: Vec<(Rect, InventorySelection, RowEvidence)>,
    view: View,
    scope: Option<HubId>,
    history: bool,
    details: Option<AgentTarget>,
    current: bool,
    support: (bool, bool),
    filter: String,
    selected: Option<InventorySelection>,
    ordered: Vec<InventorySelection>,
}

impl InventoryViewport {
    pub(crate) fn new(
        area: Rect,
        rows_area: Rect,
        tabs: Vec<(Rect, View)>,
        inventory: &Inventory,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
        selected: Option<TerminalAddress>,
        filter: &str,
    ) -> Self {
        Self {
            area,
            rows_area,
            tabs,
            rows: Vec::new(),
            view: inventory.view,
            scope: inventory.scope,
            history: inventory.history,
            details: inventory.details,
            current: inventory.current,
            support: (inventory.agents_supported, inventory.machines_supported),
            filter: filter.into(),
            selected: inventory.selected(selected),
            ordered: inventory.visible_selections(filter, terminals),
        }
    }

    pub fn area(&self) -> Rect {
        self.area
    }
    pub fn rows_area(&self) -> Rect {
        self.rows_area
    }

    pub fn current(
        &self,
        inventory: &Inventory,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
        selected: Option<TerminalAddress>,
        filter: &str,
    ) -> bool {
        self.view == inventory.view
            && self.scope == inventory.scope
            && self.history == inventory.history
            && self.details == inventory.details
            && self.current == inventory.current
            && self.support == (inventory.agents_supported, inventory.machines_supported)
            && self.filter == filter
            && self.selected == inventory.selected(selected)
            && self.ordered == inventory.visible_selections(filter, terminals)
            && self
                .rows
                .iter()
                .all(|(_, _, evidence)| evidence.matches(inventory, terminals))
    }

    pub fn hit(&self, x: u16, y: u16) -> Option<InventoryHit> {
        let position = Position::new(x, y);
        if !self.area.contains(position) {
            return None;
        }
        self.tabs
            .iter()
            .find(|(area, _)| area.contains(position))
            .map(|(_, view)| InventoryHit::Tab(*view))
            .or_else(|| {
                self.rows
                    .iter()
                    .find(|(area, _, _)| area.contains(position))
                    .map(|(_, row, _)| InventoryHit::Row(*row))
            })
    }

    pub fn scroll_target(&self, forward: bool) -> Option<InventorySelection> {
        let index = self
            .selected
            .and_then(|selected| self.ordered.iter().position(|row| *row == selected));
        let next = match index {
            None => usize::default(),
            Some(index) if forward => index
                .saturating_add(SELECTION_STEP)
                .min(self.ordered.len().saturating_sub(SELECTION_STEP)),
            Some(index) => index.saturating_sub(SELECTION_STEP),
        };
        self.ordered.get(next).copied()
    }

    pub(crate) fn bind_rows(
        &mut self,
        offset: usize,
        heights: &[usize],
        inventory: &Inventory,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) {
        let mut y = self.rows_area.y;
        for (row, height) in self.ordered.iter().zip(heights).skip(offset) {
            let available = self.rows_area.bottom().saturating_sub(y);
            let Ok(height) = u16::try_from(*height) else {
                break;
            };
            // Ratatui List draws whole items only; an unpainted tail is not a row hit.
            if height == 0 || height > available || self.rows_area.width == 0 {
                break;
            }
            if let Some(evidence) = RowEvidence::capture(*row, inventory, terminals) {
                self.rows.push((
                    Rect::new(self.rows_area.x, y, self.rows_area.width, height),
                    *row,
                    evidence,
                ));
            }
            y = y.saturating_add(height);
        }
    }
}
