use super::*;

const ADMISSION_LIMIT: usize = 1;
const SOURCE: &str = "core/src/lib.rs";
const TARGET: &str = "core/src/target.rs";
const TRAILING_TARGET: &str = "core/src/trailing.rs";

#[test]
fn duplicate_transitions_exhaust_the_queue_budget() {
    let mut modules = ModuleIndex::default();
    modules.mark_product(SOURCE);
    let mut pending = Pending::new(VecDeque::new(), ADMISSION_LIMIT, &mut modules);
    assert!(pending.push(SOURCE, state(TARGET), &mut modules));
    assert!(!pending.push(SOURCE, state(TARGET), &mut modules));
    assert!(pending.exhausted());
    assert!(modules.uncertain_files().any(|file| file == SOURCE));
    assert!(modules.uncertain_files().any(|file| file == TARGET));
}

#[test]
fn seed_budget_marks_every_unadmitted_product_root_uncertain() {
    let seeds = VecDeque::from([state(SOURCE), state(TARGET), state(TRAILING_TARGET)]);
    let mut modules = ModuleIndex::default();
    let pending = Pending::new(seeds, ADMISSION_LIMIT, &mut modules);
    assert!(pending.exhausted());
    assert!(modules.uncertain_files().any(|file| file == TARGET));
    assert!(
        modules
            .uncertain_files()
            .any(|file| file == TRAILING_TARGET)
    );
}

#[test]
fn non_product_edge_exhaustion_marks_the_product_source_uncertain() {
    let mut modules = ModuleIndex::default();
    modules.mark_product(SOURCE);
    let mut pending = Pending::new(VecDeque::new(), ADMISSION_LIMIT, &mut modules);
    assert!(pending.push(SOURCE, non_product_state(TARGET), &mut modules));
    assert!(!pending.push(SOURCE, non_product_state(TRAILING_TARGET), &mut modules));
    assert!(modules.uncertain_files().any(|file| file == SOURCE));
    assert!(!modules.uncertain_files().any(|file| file == TARGET));
    assert!(
        !modules
            .uncertain_files()
            .any(|file| file == TRAILING_TARGET)
    );
}

fn state(target: &str) -> State {
    State {
        file: target.to_owned(),
        path: Vec::new(),
        lineage: BTreeSet::new(),
        crate_root: None,
        module_search_directory: None,
        product: true,
    }
}

fn non_product_state(target: &str) -> State {
    State {
        product: false,
        ..state(target)
    }
}
