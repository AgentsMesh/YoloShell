use super::*;
use anyhow::Result;
use std::cell::RefCell;

const ACTIVE_SAVED_NAME: &str = ".prepare-active-saved";
const CREATE_BOUNDARY_ERROR: &str = "injected pre-create failure";
const EMPTY_BYTES: &[u8] = b"";
const SLOT_SAVED_NAME: &str = ".prepare-slot-saved";

type Boundary = fn() -> Result<()>;

thread_local! {
    static CREATE_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static REPLACEMENT: RefCell<Option<(PathBuf, PathBuf)>> = const { RefCell::new(None) };
}

#[test]
fn one_callback_shape_covers_every_prepare_outcome() {
    assert!(prepare_fresh_active().is_some());
    assert!(prepare_existing_active().is_some());
    assert!(prepare_recycled_slot().is_some());
    assert!(prepare_contended(Source::Active).is_none());
    assert!(prepare_contended(Source::Slot).is_none());
    assert!(prepare_replaced(Source::Active).is_none());
    assert!(prepare_replaced(Source::Slot).is_none());
    assert!(prepare_colliding_create().is_none());
    assert!(prepare_create_hook_error().is_err());
    assert!(prepare_unsafe(Source::Active).is_err());
    assert!(prepare_unsafe(Source::Slot).is_err());
}

fn prepare_fresh_active() -> Option<(File, crate::paths::process_file_lock::ProcessFileLock)> {
    let (_root, entry) = fixture();
    prepare(&entry, noop, noop)
}

fn prepare_existing_active() -> Option<(File, crate::paths::process_file_lock::ProcessFileLock)> {
    let (_root, entry) = fixture();
    write_private(&named_path(&entry, &name(ACTIVE_NAME)), ORIGINAL_BYTES);
    prepare(&entry, noop, noop)
}

fn prepare_recycled_slot() -> Option<(File, crate::paths::process_file_lock::ProcessFileLock)> {
    let (_root, entry) = fixture();
    let slot = name(SLOT_NAME);
    let slot_path = named_path(&entry, &slot);
    write_private(&slot_path, ORIGINAL_BYTES);
    let prepared = prepare(&entry, noop, noop);
    assert!(!slot_path.exists());
    assert_eq!(
        fs::read(named_path(&entry, &name(ACTIVE_NAME))).unwrap(),
        EMPTY_BYTES
    );
    prepared
}

fn prepare_contended(
    source: Source,
) -> Option<(File, crate::paths::process_file_lock::ProcessFileLock)> {
    let (_root, entry) = fixture();
    let active = name(ACTIVE_NAME);
    let slot = name(SLOT_NAME);
    let path = named_path(&entry, source.name(&active, &slot));
    write_private(&path, ORIGINAL_BYTES);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let owner = private_file_temp_prepare::lock_or_retry(&file)
        .unwrap()
        .unwrap();
    let prepared = prepare(&entry, noop, noop);
    drop(owner);
    prepared
}

fn prepare_replaced(
    source: Source,
) -> Option<(File, crate::paths::process_file_lock::ProcessFileLock)> {
    let (root, entry) = fixture();
    let active = name(ACTIVE_NAME);
    let slot = name(SLOT_NAME);
    let path = named_path(&entry, source.name(&active, &slot));
    let saved_name = match source {
        Source::Active => ACTIVE_SAVED_NAME,
        Source::Slot => SLOT_SAVED_NAME,
    };
    let saved = root.path().join(saved_name);
    write_private(&path, ORIGINAL_BYTES);
    REPLACEMENT.with(|state| *state.borrow_mut() = Some((path.clone(), saved.clone())));
    let prepared = prepare(&entry, replace_opened, noop);
    assert_eq!(fs::read(saved).unwrap(), ORIGINAL_BYTES);
    assert_eq!(fs::read(path).unwrap(), REPLACEMENT_BYTES);
    prepared
}

fn prepare_colliding_create() -> Option<(File, crate::paths::process_file_lock::ProcessFileLock)> {
    let (_root, entry) = fixture();
    let path = named_path(&entry, &name(ACTIVE_NAME));
    CREATE_PATH.with(|state| *state.borrow_mut() = Some(path.clone()));
    let prepared = prepare(&entry, noop, create_competitor);
    assert_eq!(fs::read(path).unwrap(), REPLACEMENT_BYTES);
    prepared
}

fn prepare_create_hook_error() -> Result<()> {
    let (_root, entry) = fixture();
    prepare_result(&entry, noop, fail_create).map(|_| ())
}

fn prepare_unsafe(source: Source) -> Result<()> {
    let (_root, entry) = fixture();
    let active = name(ACTIVE_NAME);
    let slot = name(SLOT_NAME);
    fs::create_dir(named_path(&entry, source.name(&active, &slot))).unwrap();
    prepare_result(&entry, noop, noop).map(|_| ())
}

fn prepare(
    entry: &PrivateFileEntry,
    after_open: Boundary,
    before_create: Boundary,
) -> Option<(File, crate::paths::process_file_lock::ProcessFileLock)> {
    prepare_result(entry, after_open, before_create).unwrap()
}

fn prepare_result(
    entry: &PrivateFileEntry,
    mut after_open: Boundary,
    mut before_create: Boundary,
) -> Result<Option<(File, crate::paths::process_file_lock::ProcessFileLock)>> {
    let mut before_truncate: Boundary = noop;
    entry.prepare_active(
        &name(ACTIVE_NAME),
        &name(SLOT_NAME),
        &mut after_open,
        &mut before_truncate,
        &mut before_create,
    )
}

fn noop() -> Result<()> {
    Ok(())
}

fn create_competitor() -> Result<()> {
    let path = CREATE_PATH.with(|state| state.borrow_mut().take().unwrap());
    write_private(&path, REPLACEMENT_BYTES);
    Ok(())
}

fn fail_create() -> Result<()> {
    anyhow::bail!(CREATE_BOUNDARY_ERROR)
}

fn replace_opened() -> Result<()> {
    let (path, saved) = REPLACEMENT.with(|state| state.borrow_mut().take().unwrap());
    fs::rename(&path, saved)?;
    write_private(&path, REPLACEMENT_BYTES);
    Ok(())
}
