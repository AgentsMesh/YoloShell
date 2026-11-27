use crate::State;
use yolo_hub_identity::TerminalAddress;
use yolo_tui_inventory::View;

impl State {
    pub fn visible(&self) -> Vec<TerminalAddress> {
        self.inventory
            .visible_terminals(&self.filter, &self.terminals)
    }

    pub fn reconcile_selection(&mut self) {
        let ended = self.ended_epoch_map();
        self.inventory.observe_ended(ended);
        self.inventory.reconcile(&self.filter, &self.terminals);
        let visible = self.visible();
        if self
            .selected
            .is_none_or(|selected| !visible.contains(&selected))
        {
            self.selected = visible.first().copied();
        }
    }

    pub fn move_selection(&mut self, forward: bool) {
        if self.inventory.view != View::Terminals {
            self.inventory
                .move_selection(forward, &self.filter, &self.terminals);
            return;
        }
        let visible = self.visible();
        if visible.is_empty() {
            self.selected = None;
            return;
        }
        let index = self
            .selected
            .and_then(|selected| visible.iter().position(|address| *address == selected))
            .unwrap_or_default();
        let next = if forward {
            (index + 1).min(visible.len() - 1)
        } else {
            index.saturating_sub(1)
        };
        self.selected = Some(visible[next]);
    }

    pub fn focused_terminal(&self) -> Option<TerminalAddress> {
        self.tab_layout()
            .panes()
            .into_iter()
            .find(|pane| pane.id == self.tab_layout().focused())
            .and_then(|pane| pane.terminal)
    }
}
