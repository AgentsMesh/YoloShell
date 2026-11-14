use crate::State;
use yolo_hub_terminal_protocol::TerminalSummary;

impl State {
    pub(crate) fn replace_session(&mut self, session: TerminalSummary) {
        let address = session.fields().address;
        let previous = self
            .terminals
            .get(&address)
            .and_then(|current| current.fields().runtime)
            .map(|runtime| runtime.target);
        let current = session.fields().runtime.map(|runtime| runtime.target);
        let writer_replaced = self
            .writers
            .get(&address)
            .is_some_and(|writer| Some(writer.grant.fields().target) != current);
        if previous != current || writer_replaced {
            self.retire_runtime(address);
        }
        self.detect_resized_observers(std::slice::from_ref(&session));
        self.terminals.insert(address, session);
        self.reconcile_selection();
        self.retain_writer_runtime_facts();
        self.sync_query_authority();
        crate::tab_liveness::retire_dead(self);
    }

    pub fn replace_terminals(&mut self, terminals: Vec<TerminalSummary>) {
        let terminals = terminals;
        for session in &terminals {
            if session.fields().runtime.is_some_and(|runtime| {
                !matches!(
                    runtime.status,
                    yolo_hub_terminal_protocol::RuntimeStatus::Unreachable
                        | yolo_hub_terminal_protocol::RuntimeStatus::Exited
                        | yolo_hub_terminal_protocol::RuntimeStatus::Lost
                )
            }) {
                self.clear_offline(session.fields().address);
            }
        }
        let terminal_map = terminals
            .iter()
            .map(|session| (session.fields().address, session.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut retired = self
            .terminals
            .iter()
            .filter_map(|(address, session)| {
                let previous = session.fields().runtime.map(|runtime| runtime.target);
                let current = terminal_map
                    .get(address)
                    .and_then(|session| session.fields().runtime)
                    .map(|runtime| runtime.target);
                (previous != current).then_some(*address)
            })
            .collect::<std::collections::BTreeSet<_>>();
        retired.extend(
            self.writers
                .iter()
                .filter_map(|(address, writer)| {
                    let current = terminal_map
                        .get(address)
                        .and_then(|session| session.fields().runtime)
                        .map(|runtime| runtime.target);
                    (current != Some(writer.grant.fields().target)).then_some(*address)
                })
                .collect::<Vec<_>>(),
        );
        for address in retired {
            self.retire_runtime(address);
        }
        self.detect_resized_observers(&terminals);
        self.terminals = terminals
            .into_iter()
            .map(|session| (session.fields().address, session))
            .collect::<std::collections::BTreeMap<_, _>>();
        self.reconcile_selection();
        self.retain_writer_runtime_facts();
        self.sync_query_authority();
        crate::tab_liveness::retire_dead(self);
    }
}
