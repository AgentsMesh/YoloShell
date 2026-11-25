use super::*;
use std::cell::RefCell;
use std::os::unix::fs::PermissionsExt;
use std::thread::LocalKey;

const CHILD_NAME: &str = "branch-child";
const DIRECTORY_CHANGED_DETAIL: &str = "private directory changed while opening";
const DISPLACED_NAME: &str = "displaced-directory";
const MANAGED_NAME: &str = "managed-directory";
const NON_NORMAL_PATH: &str = "/tmp/../yolo-directory";
const OWNER_MISMATCH_DETAIL: &str = "private directory owner mismatch";
const PATH_CHANGED_DETAIL: &str = "directory path changed";
const READ_ONLY_DIRECTORY_MODE: u32 = 0o500;
const ROOT_PATH: &str = "/";

type DirectoryHook = Box<dyn FnOnce()>;
type HookSlot = RefCell<Option<DirectoryHook>>;

thread_local! {
    static AFTER_FIRST_OPEN: HookSlot = RefCell::new(None);
    static BEFORE_DIRECTORY_CREATE: HookSlot = RefCell::new(None);
    static BEFORE_ENSURE_REOPEN: HookSlot = RefCell::new(None);
}

pub(super) fn after_first_open() {
    run_hook(&AFTER_FIRST_OPEN);
}

pub(super) fn before_directory_create() {
    run_hook(&BEFORE_DIRECTORY_CREATE);
}

pub(super) fn before_ensure_reopen() {
    run_hook(&BEFORE_ENSURE_REOPEN);
}

#[test]
fn open_and_ensure_reject_replaced_directory_paths() {
    for boundary in [ReplacementBoundary::Open, ReplacementBoundary::Ensure] {
        let root = tempfile::tempdir().unwrap();
        let managed = root.path().join(MANAGED_NAME);
        PrivateDirectory::ensure(&managed).unwrap();
        let displaced = root.path().join(DISPLACED_NAME);
        boundary.install(managed.clone(), displaced);

        let error = boundary.run(&managed).unwrap_err();

        assert!(error.to_string().contains(DIRECTORY_CHANGED_DETAIL));
    }
}

#[test]
fn concurrent_directory_creation_is_opened_without_claiming_creation() {
    let root = tempfile::tempdir().unwrap();
    let parent = File::open(root.path()).unwrap();
    let child = root.path().join(CHILD_NAME);
    set_hook(&BEFORE_DIRECTORY_CREATE, move || {
        std::fs::create_dir(&child).unwrap();
        set_mode(&child, PRIVATE_DIRECTORY_MODE);
    });

    let (opened, created) = open_or_create(
        &parent,
        &entry_c_string(OsStr::new(CHILD_NAME)).unwrap(),
        root.path(),
    )
    .unwrap();

    assert!(!created);
    DirectoryIdentity::inspect_private(&opened.metadata().unwrap(), root.path()).unwrap();
}

#[test]
fn directory_creation_failure_is_returned_without_mutating_the_parent_mode() {
    if unsafe { libc::geteuid() } == crate::paths::constants::ROOT_USER_ID {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let parent = File::open(root.path()).unwrap();
    let root_path = root.path().to_path_buf();
    set_hook(&BEFORE_DIRECTORY_CREATE, move || {
        set_mode(&root_path, READ_ONLY_DIRECTORY_MODE);
    });

    let result = open_or_create(
        &parent,
        &entry_c_string(OsStr::new(CHILD_NAME)).unwrap(),
        root.path(),
    );
    set_mode(root.path(), PRIVATE_DIRECTORY_MODE);

    assert!(result.is_err());
    assert!(!root.path().join(CHILD_NAME).exists());
}

#[test]
fn owner_and_descriptor_identity_checks_fail_closed() {
    if unsafe { libc::geteuid() } != crate::paths::constants::ROOT_USER_ID {
        let root = File::open(ROOT_PATH).unwrap();
        let error = verify_normalizable_private(&root, Path::new(ROOT_PATH)).unwrap_err();
        assert!(error.to_string().contains(OWNER_MISMATCH_DETAIL));
    }

    let root = tempfile::tempdir().unwrap();
    let managed = root.path().join(MANAGED_NAME);
    PrivateDirectory::ensure(&managed).unwrap();
    let retained = File::open(&managed).unwrap();
    std::fs::rename(&managed, root.path().join(DISPLACED_NAME)).unwrap();
    std::fs::create_dir(&managed).unwrap();
    set_mode(&managed, PRIVATE_DIRECTORY_MODE);
    let verified = crate::paths::platform::platform_path(&managed);

    let error = verify_descriptor_path(&retained, &verified).unwrap_err();

    assert!(error.to_string().contains(PATH_CHANGED_DETAIL), "{error:#}");
}

#[test]
fn component_parser_rejects_parent_traversal() {
    assert!(normal_components(Path::new(NON_NORMAL_PATH)).is_err());
}

#[derive(Clone, Copy)]
enum ReplacementBoundary {
    Open,
    Ensure,
}

impl ReplacementBoundary {
    fn install(self, managed: PathBuf, displaced: PathBuf) {
        let hook = move || replace_directory(&managed, &displaced);
        match self {
            Self::Open => set_hook(&AFTER_FIRST_OPEN, hook),
            Self::Ensure => set_hook(&BEFORE_ENSURE_REOPEN, hook),
        }
    }

    fn run(self, managed: &Path) -> Result<PrivateDirectory> {
        match self {
            Self::Open => PrivateDirectory::open(managed),
            Self::Ensure => PrivateDirectory::ensure(managed),
        }
    }
}

fn replace_directory(managed: &Path, displaced: &Path) {
    std::fs::rename(managed, displaced).unwrap();
    std::fs::create_dir(managed).unwrap();
    set_mode(managed, PRIVATE_DIRECTORY_MODE);
}

fn set_mode(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn set_hook(slot: &'static LocalKey<HookSlot>, hook: impl FnOnce() + 'static) {
    slot.with(|current| assert!(current.borrow_mut().replace(Box::new(hook)).is_none()));
}

fn run_hook(slot: &'static LocalKey<HookSlot>) {
    if let Some(hook) = slot.with(|current| current.borrow_mut().take()) {
        hook();
    }
}

#[path = "directory_coverage_matrix.rs"]
mod coverage_matrix;
