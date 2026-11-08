use super::ModuleIndex;
use std::collections::{BTreeSet, VecDeque};

pub(super) struct State {
    pub file: String,
    pub path: Vec<String>,
    pub lineage: BTreeSet<String>,
    pub crate_root: Option<String>,
    pub module_search_directory: Option<String>,
    pub product: bool,
}

pub(super) struct Pending {
    states: VecDeque<State>,
    admitted: usize,
    limit: usize,
    exhausted: bool,
}

impl Pending {
    pub fn new(seeds: VecDeque<State>, limit: usize, modules: &mut ModuleIndex) -> Self {
        let mut pending = Self {
            states: VecDeque::new(),
            admitted: 0,
            limit,
            exhausted: false,
        };
        for state in seeds {
            if pending.exhausted {
                if state.product {
                    modules.mark_uncertain(&state.file);
                }
                continue;
            }
            let source = state.file.clone();
            pending.push(&source, state, modules);
        }
        pending
    }

    pub fn pop_front(&mut self) -> Option<State> {
        self.states.pop_front()
    }

    pub fn push(&mut self, source: &str, state: State, modules: &mut ModuleIndex) -> bool {
        if self.admitted >= self.limit {
            if modules.is_product(source) {
                modules.mark_uncertain(source);
            }
            if state.product {
                modules.mark_uncertain(&state.file);
            }
            for queued in self.states.iter().filter(|queued| queued.product) {
                modules.mark_uncertain(&queued.file);
            }
            self.exhausted = true;
            return false;
        }
        self.admitted += 1;
        self.states.push_back(state);
        true
    }

    pub fn exhausted(&self) -> bool {
        self.exhausted
    }
}

#[cfg(test)]
#[path = "../../unit_tests/graph_pending_tests.rs"]
mod tests;
