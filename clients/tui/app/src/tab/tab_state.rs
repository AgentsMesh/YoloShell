use crate::{Mode, State};
use std::collections::BTreeSet;
use yolo_hub_identity::TerminalAddress;
use yolo_tui_tab::{TabCatalog, TabViewMode};

impl State {
    /// Installs a tab catalog only after terminal-binding admission has
    /// removed every pane that can no longer accept input.
    pub(crate) fn replace_tabs(&mut self, tabs: TabCatalog) -> crate::tab_binding::Admission {
        let (tabs, admission) = crate::tab_binding::admit(self, tabs);
        self.tab_revision = self.tab_revision.saturating_add(1);
        self.tabs = tabs;
        self.inventory.remember_workspace_machines(self.tabs.entries().iter().map(|entry| entry.machine).collect());
        admission
    }
    pub(crate) fn sync_active_tab_view(&mut self) {
        let view = if matches!(self.mode, Mode::Terminal | Mode::Chat) {
            TabViewMode::Terminal
        } else {
            TabViewMode::Inventory
        };
        if self.tabs.active_view() != view {
            self.tab_revision = self.tab_revision.saturating_add(1);
            self.tabs.replace_active_view(view);
        }
    }

    pub(crate) fn tab_snapshot(&self) -> TabCatalog {
        let mut tabs = self.tabs.clone();
        tabs.replace_active_view(if matches!(self.mode, Mode::Terminal | Mode::Chat) {
            TabViewMode::Terminal
        } else {
            TabViewMode::Inventory
        });
        tabs
    }

    pub(crate) fn bound_terminals(&self) -> BTreeSet<TerminalAddress> {
        self.tabs
            .entries()
            .iter()
            .filter(|entry| entry.id != self.tabs.active().id)
            .flat_map(|entry| entry.view.layout.panes())
            .chain(self.tab_layout().panes())
            .filter_map(|pane| pane.terminal.filter(|address| !crate::constants::is_self_terminal(address.terminal_id.as_uuid())))
            .collect()
    }

    pub(crate) fn activate_tab_collection(&mut self) {
        self.mode = if self.mode == Mode::Document && self.tabs.active_document().is_some() {
            Mode::Document
        } else {
            match self.tabs.active_view() {
                TabViewMode::Inventory => Mode::Inventory,
                TabViewMode::Terminal => Mode::Terminal,
            }
        };
        self.selected = self.focused_terminal();
        self.reconcile_selection();
        self.prefix = false;
        self.pointer = crate::pointer::Pointer::default();
        self.pane_view = crate::pane_view::PaneView::default();
        crate::selection_actions::cancel(self);
        self.presentation = crate::presentation::Presentation::default();
        self.confirmation = None;
        self.sync_query_authority();
    }
}
