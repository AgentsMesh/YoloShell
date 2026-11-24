use super::*;
use std::collections::VecDeque;
use yolo_architecture_syntax::targeted_tokens::tokenize;

const ROOT: &str = "core/src/lib.rs";
const CHILD: &str = "core/src/child.rs";
const STATE_BUDGET: usize = 8;

fn transition(text: &str, product: bool, cycle: bool, limit: usize) -> (ModuleIndex, Pending) {
    let mut modules = ModuleIndex::default();
    if product {
        modules.mark_product(ROOT);
    }
    let mut pending = Pending::new(VecDeque::new(), limit, &mut modules);
    let mut lineage = BTreeSet::from([ROOT.to_owned()]);
    if cycle {
        lineage.insert(CHILD.to_owned());
    }
    let state = State {
        file: ROOT.into(),
        path: vec![ROOT.into()],
        lineage,
        crate_root: Some(ROOT.into()),
        module_search_directory: None,
        product,
    };
    extend(
        &state,
        &tokenize(text),
        &BTreeSet::from([ROOT.into(), CHILD.into()]),
        &mut modules,
        &mut pending,
    );
    (modules, pending)
}

#[test]
fn module_and_include_edges_preserve_product_context_and_uncertain_ancestry() {
    for text in ["mod child;", "include!(\"child.rs\");"] {
        for product in [false, true] {
            let (modules, mut pending) = transition(text, product, false, STATE_BUDGET);
            assert_eq!(pending.pop_front().unwrap().product, product);
            assert!(modules.uncertain_files().next().is_none());
            for annotation in ["#[cfg(test)]", "#[cfg_attr(unix, path = \"child.rs\")]"] {
                let (modules, mut pending) = transition(
                    &format!("{annotation} {text}"),
                    product,
                    false,
                    STATE_BUDGET,
                );
                let is_test = annotation.contains("cfg(test)");
                assert_eq!(pending.pop_front().unwrap().product, product && !is_test);
                assert_eq!(
                    modules.uncertain_files().any(|file| file == ROOT),
                    product && !is_test
                );
            }
            let (modules, mut pending) = transition(
                &format!("#[cfg_attr(unix, cfg(test))] mod outer {{ {text} }}"),
                product,
                false,
                STATE_BUDGET,
            );
            assert_eq!(modules.uncertain_files().any(|file| file == ROOT), product);
            if text.starts_with("include") {
                assert_eq!(pending.pop_front().unwrap().product, product);
            }
        }
    }
}

#[test]
fn unavailable_non_item_and_cyclic_edges_are_uncertain_only_in_product_context() {
    for product in [false, true] {
        for text in [
            "mod absent;",
            "include!(\"absent.rs\");",
            "include!(dynamic());",
            "fn operation() { mod child; }",
            "fn operation() { include!(\"child.rs\"); }",
        ] {
            let (modules, mut pending) = transition(text, product, false, STATE_BUDGET);
            assert!(pending.pop_front().is_none(), "{text}");
            assert_eq!(
                modules.uncertain_files().any(|file| file == ROOT),
                product,
                "{text}"
            );
        }
        for text in ["mod child;", "include!(\"child.rs\");"] {
            let (modules, mut pending) = transition(text, product, true, STATE_BUDGET);
            assert!(pending.pop_front().is_none());
            assert_eq!(modules.uncertain_files().any(|file| file == CHILD), product);
            assert_eq!(modules.uncertain_files().any(|file| file == ROOT), product);
        }
    }
}

#[test]
fn disabled_context_and_budget_exhaustion_do_not_admit_following_edges() {
    for text in [
        "#[cfg(any())] mod outer { mod child; }",
        "#[cfg(any())] mod outer { include!(\"child.rs\"); }",
    ] {
        let (modules, mut pending) = transition(text, true, false, STATE_BUDGET);
        assert!(pending.pop_front().is_none());
        assert!(modules.uncertain_files().next().is_none());
    }
    for text in [
        "mod child; include!(\"child.rs\");",
        "include!(\"child.rs\"); include!(\"child.rs\");",
    ] {
        let (modules, mut pending) = transition(text, true, false, 0);
        assert!(pending.exhausted());
        assert!(pending.pop_front().is_none());
        assert!(modules.uncertain_files().any(|file| file == ROOT));
        assert!(modules.uncertain_files().any(|file| file == CHILD));
    }
}
