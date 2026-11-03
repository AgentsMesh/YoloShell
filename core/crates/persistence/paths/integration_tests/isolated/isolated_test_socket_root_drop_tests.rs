use super::*;
use std::cell::Cell;
use std::rc::Rc;

const FINALIZATION_FAILURE: &str = "injected root finalization failure";
const EXPLICIT_ATTEMPTS: usize = 1;
const RETRIED_ATTEMPTS: usize = 2;

#[test]
fn failed_explicit_finalization_is_not_repeated_by_drop() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let path = root.path().to_path_buf();
    let attempts = Rc::new(Cell::new(0));
    let observed = Rc::clone(&attempts);
    let hook = hooks::set_fault_hook(Box::new(move |point, _| {
        if point == hooks::FaultPoint::RetireBeforeRename {
            observed.set(observed.get() + 1);
            anyhow::bail!(FINALIZATION_FAILURE);
        }
        Ok(())
    }));

    assert!(root.finalize().is_err());
    drop(root);
    let attempts_after_drop = attempts.get();
    let evidence_preserved = path.is_dir();
    drop(hook);
    // A distinct owner may take over; no authority is silently discarded.
    IsolatedTestSocketRoot::adopt(&path)
        .unwrap()
        .finalize()
        .unwrap();
    assert_eq!(attempts_after_drop, EXPLICIT_ATTEMPTS);
    assert!(evidence_preserved);
    assert!(!path.exists());
}

#[test]
fn explicit_retry_still_finalizes_after_a_transient_failure() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let path = root.path().to_path_buf();
    let attempts = Rc::new(Cell::new(0));
    let observed = Rc::clone(&attempts);
    let hook = hooks::set_fault_hook(Box::new(move |point, _| {
        if point == hooks::FaultPoint::RetireBeforeRename {
            let previous = observed.replace(observed.get() + 1);
            if previous == 0 {
                anyhow::bail!(FINALIZATION_FAILURE);
            }
        }
        Ok(())
    }));

    assert!(root.finalize().is_err());
    assert!(path.is_dir());
    root.finalize().unwrap();
    drop(root);
    assert_eq!(attempts.get(), RETRIED_ATTEMPTS);
    assert!(!path.exists());
    drop(hook);
}

#[test]
fn unattempted_adopted_owner_finalizes_on_early_return() {
    let mut creator = IsolatedTestSocketRoot::create().unwrap();
    let path = creator.path().to_path_buf();
    let adopted = IsolatedTestSocketRoot::adopt(&path).unwrap();
    let hook = hooks::set_fault_hook(Box::new(|point, _| {
        if point == hooks::FaultPoint::RetireBeforeRename {
            anyhow::bail!(FINALIZATION_FAILURE);
        }
        Ok(())
    }));
    assert!(creator.finalize().is_err());
    drop(creator);
    drop(hook);
    assert!(path.is_dir());
    drop(adopted);
    assert!(!path.exists());
}
