use super::*;
use std::cell::{Cell, RefCell};
use std::fs::{self, File};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::rc::Rc;

const INJECTED_ERROR: &str = "injected isolated root fault";
const REPLACEMENT_MARKER: &str = "replacement-marker";
const DISPLACED_PREFIX: &str = "yd-fault-";

#[test]
fn creation_failure_after_mkdir_removes_exact_empty_root() {
    let created = Rc::new(RefCell::new(None));
    let observed = Rc::clone(&created);
    let _hook = hooks::set_fault_hook(Box::new(move |point, path| {
        if point == hooks::FaultPoint::CreationAfterMkdir {
            *observed.borrow_mut() = Some(path.to_path_buf());
            anyhow::bail!(INJECTED_ERROR);
        }
        Ok(())
    }));

    assert!(IsolatedTestSocketRoot::create().is_err());
    let path = created.borrow().clone().unwrap();
    assert!(!path.exists());
}

#[test]
fn creation_repairs_restrictive_initial_mode() {
    let changed = Rc::new(Cell::new(false));
    let observed = Rc::clone(&changed);
    let _hook = hooks::set_fault_hook(Box::new(move |point, path| {
        if point == hooks::FaultPoint::CreationAfterMkdir {
            fs::set_permissions(path, fs::Permissions::from_mode(0))?;
            observed.set(true);
        }
        Ok(())
    }));

    let mut root = IsolatedTestSocketRoot::create().unwrap();
    assert!(changed.get());
    assert_eq!(
        root.path().metadata().unwrap().permissions().mode() & PERMISSION_MODE_MASK,
        PRIVATE_DIRECTORY_MODE,
    );
    root.finalize().unwrap();
}

#[test]
fn replacement_between_verify_and_rename_is_restored() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let active = root.path().to_path_buf();
    let displaced = active.with_file_name(format!(
        "{DISPLACED_PREFIX}{}",
        uuid::Uuid::new_v4().simple()
    ));
    let replacement_marker = active.join(REPLACEMENT_MARKER);
    let injected = Rc::new(Cell::new(false));
    let observed = Rc::clone(&injected);
    let active_for_hook = active.clone();
    let displaced_for_hook = displaced.clone();
    let hook = hooks::set_fault_hook(Box::new(move |point, _| {
        if point == hooks::FaultPoint::RetireBeforeRename && !observed.replace(true) {
            fs::rename(&active_for_hook, &displaced_for_hook)?;
            fs::DirBuilder::new()
                .mode(PRIVATE_DIRECTORY_MODE)
                .create(&active_for_hook)?;
            File::create(active_for_hook.join(REPLACEMENT_MARKER))?;
        }
        Ok(())
    }));

    assert!(root.finalize().is_err());
    assert!(replacement_marker.is_file());
    drop(hook);
    fs::remove_file(&replacement_marker).unwrap();
    fs::remove_dir(&active).unwrap();
    fs::rename(&displaced, &active).unwrap();
    root.finalize().unwrap();
    assert!(!active.exists());
}

#[test]
fn post_rename_failures_preserve_retryable_retired_state() {
    for point in [
        hooks::FaultPoint::RetireAfterRename,
        hooks::FaultPoint::RetireBeforeOpen,
    ] {
        assert_retryable_retirement_failure(point);
    }
}

fn assert_retryable_retirement_failure(fault: hooks::FaultPoint) {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let active = root.path().to_path_buf();
    let injected = Rc::new(Cell::new(false));
    let observed = Rc::clone(&injected);
    let hook = hooks::set_fault_hook(Box::new(move |point, _| {
        if point == fault && !observed.replace(true) {
            anyhow::bail!(INJECTED_ERROR);
        }
        Ok(())
    }));

    assert!(root.finalize().is_err());
    let retired = root.path().to_path_buf();
    assert!(injected.get());
    assert!(!active.exists());
    assert!(retired.is_dir());
    drop(hook);
    root.finalize().unwrap();
    assert!(!retired.exists());
}

include!("isolated_test_socket_root_branch_tests.rs");
