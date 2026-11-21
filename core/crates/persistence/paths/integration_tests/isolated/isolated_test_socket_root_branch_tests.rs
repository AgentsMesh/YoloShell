use std::os::unix::ffi::OsStrExt;

const COLLISION_ERROR_DETAIL: &str = "collision budget exhausted";
const CREATION_DISPLACED_PREFIX: &str = "creation-displaced-";
const FAILED_CREATION_NAME: &str = "denied-isolated-root";
const INJECTED_NAME_ERROR: &str = "injected isolated root name failure";
const MISSING_PARENT_NAME: &str = "missing-isolated-parent";
const NUL_PARENT_BYTES: &[u8] = b"/tmp/isolated\0parent";
const READ_ONLY_PARENT_MODE: u32 = 0o500;
const RETIRE_DISPLACED_PREFIX: &str = "retire-displaced-";
const RETIREMENT_MARKER_NAME: &str = "retirement-race-marker";

#[test]
fn creation_exhausts_real_collisions_and_rejects_a_missing_parent() {
    let temporary = tempfile::tempdir().unwrap();
    assert!(create::open_parent_at(&temporary.path().join(MISSING_PARENT_NAME)).is_err());

    let collision = create::random_name(ISOLATED_TEST_SOCKET_PREFIX).unwrap();
    let collision_path = socket_parent_path(&collision);
    create_private_directory(&collision_path);
    let hook_name = collision.clone();
    let hook = hooks::set_name_hook(Box::new(move |_| Ok(Some(hook_name.clone()))));

    let error = IsolatedTestSocketRoot::create().err().unwrap();

    assert!(error.to_string().contains(COLLISION_ERROR_DETAIL));
    drop(hook);
    fs::remove_dir(collision_path).unwrap();
}

#[test]
fn generated_name_and_parent_encoding_errors_propagate() {
    let hook = hooks::set_name_hook(Box::new(|_| anyhow::bail!(INJECTED_NAME_ERROR)));
    let error = IsolatedTestSocketRoot::create().err().unwrap();
    assert!(error.to_string().contains(INJECTED_NAME_ERROR));
    drop(hook);

    let invalid = std::ffi::OsStr::from_bytes(NUL_PARENT_BYTES);
    assert!(create::open_parent_at(Path::new(invalid)).is_err());
}

#[test]
fn creation_returns_a_real_parent_permission_failure() {
    if unsafe { libc::geteuid() } == crate::paths::constants::ROOT_USER_ID {
        return;
    }
    let temporary = tempfile::tempdir().unwrap();
    let parent = File::open(temporary.path()).unwrap();
    fs::set_permissions(
        temporary.path(),
        fs::Permissions::from_mode(READ_ONLY_PARENT_MODE),
    )
    .unwrap();
    let name = CString::new(FAILED_CREATION_NAME).unwrap();

    let result = create::create_directory(&parent, &name);
    fs::set_permissions(
        temporary.path(),
        fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();

    assert!(result.is_err());
    assert!(!temporary.path().join(FAILED_CREATION_NAME).exists());
}

#[test]
fn creation_detects_disappearance_before_opening_the_created_root() {
    let displaced = Rc::new(RefCell::new(None));
    let observed = Rc::clone(&displaced);
    let hook = hooks::set_fault_hook(Box::new(move |point, path| {
        if point == hooks::FaultPoint::CreationBeforeOpen {
            let target = sibling_path(path, CREATION_DISPLACED_PREFIX);
            fs::rename(path, &target)?;
            *observed.borrow_mut() = Some(target);
        }
        Ok(())
    }));

    assert!(IsolatedTestSocketRoot::create().is_err());

    drop(hook);
    fs::remove_dir(displaced.borrow().as_ref().unwrap()).unwrap();
}

#[test]
fn retirement_exhausts_real_destination_collisions_then_retries() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let collision = create::random_name(ISOLATED_TEST_SOCKET_RETIRED_PREFIX).unwrap();
    let collision_path = socket_parent_path(&collision);
    create_private_directory(&collision_path);
    let hook_name = collision.clone();
    let hook = hooks::set_name_hook(Box::new(move |_| Ok(Some(hook_name.clone()))));

    let error = root.finalize().unwrap_err();

    assert!(error.to_string().contains(COLLISION_ERROR_DETAIL));
    drop(hook);
    fs::remove_dir(collision_path).unwrap();
    root.finalize().unwrap();
}

#[test]
fn retirement_reports_source_disappearance_without_losing_retry_authority() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let active = root.path().to_path_buf();
    let displaced = sibling_path(&active, RETIRE_DISPLACED_PREFIX);
    let active_for_hook = active.clone();
    let displaced_for_hook = displaced.clone();
    let hook = hooks::set_fault_hook(Box::new(move |point, _| {
        if point == hooks::FaultPoint::RetireBeforeRename {
            fs::rename(&active_for_hook, &displaced_for_hook)?;
        }
        Ok(())
    }));

    assert!(root.finalize().is_err());

    drop(hook);
    fs::rename(displaced, active).unwrap();
    root.finalize().unwrap();
}

#[test]
fn post_verification_entries_fence_empty_check_and_root_unlink() {
    assert_post_verification_entry(hooks::FaultPoint::RetireBeforeRootUnlink);
}

fn assert_post_verification_entry(point: hooks::FaultPoint) {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let marker = Rc::new(RefCell::new(None));
    let observed = Rc::clone(&marker);
    let hook = hooks::set_fault_hook(Box::new(move |current, path| {
        if current == point && observed.borrow().is_none() {
            let path = path.join(RETIREMENT_MARKER_NAME);
            File::create(&path)?;
            *observed.borrow_mut() = Some(path);
        }
        Ok(())
    }));

    assert!(root.finalize().is_err());
    let marker = marker.borrow().clone().unwrap();
    assert!(marker.is_file());
    drop(hook);
    fs::remove_file(marker).unwrap();
    root.finalize().unwrap();
}

fn create_private_directory(path: &Path) {
    fs::DirBuilder::new()
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(path)
        .unwrap();
}

fn sibling_path(path: &Path, prefix: &str) -> PathBuf {
    path.with_file_name(format!("{prefix}{}", uuid::Uuid::new_v4().simple()))
}

fn socket_parent_path(name: &CString) -> PathBuf {
    Path::new(ISOLATED_TEST_SOCKET_PARENT).join(std::ffi::OsStr::from_bytes(name.to_bytes()))
}
