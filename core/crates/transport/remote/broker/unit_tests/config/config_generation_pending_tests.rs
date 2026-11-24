use super::*;
use std::fs;
use std::os::fd::RawFd;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const SENTINEL_FILE_NAME: &str = "must-survive";
const SENTINEL_BYTES: &[u8] = b"unrelated";
const PENDING_BYTES: &[u8] = b"pending";
const UNSAFE_STAGING_MODE: u32 = 0o755;
const INVALID_DESCRIPTOR: RawFd = -1;
const MISSING_ENTRY_NAME: &str = "missing-generation-entry";
const TARGET_ENTRY_NAME: &str = "unreachable-generation-target";

#[test]
fn pending_cleanup_only_mutates_the_held_directory() {
    let fixture = PendingFixture::new();
    let moved = fixture.base.path().join("moved");
    let guard = PendingDirectory::create(&fixture.root).unwrap();
    guard
        .write_file(&entry_name(GENERATION_CONFIG_FILE).unwrap(), PENDING_BYTES)
        .unwrap();
    fs::rename(&fixture.slot, &moved).unwrap();
    fs::create_dir(&fixture.slot).unwrap();
    fs::write(fixture.slot.join(SENTINEL_FILE_NAME), SENTINEL_BYTES).unwrap();

    drop(guard);

    assert_eq!(
        fs::read(fixture.slot.join(SENTINEL_FILE_NAME)).unwrap(),
        SENTINEL_BYTES
    );
    assert!(!moved.join(GENERATION_CONFIG_FILE).exists());
    assert!(moved.is_dir());
}

#[test]
fn fixed_staging_slot_is_cleaned_and_recycled() {
    let fixture = PendingFixture::new();
    let name = entry_name(GENERATION_CONFIG_FILE).unwrap();
    let first = PendingDirectory::create(&fixture.root).unwrap();
    first.write_file(&name, PENDING_BYTES).unwrap();
    assert!(first.write_file(&name, PENDING_BYTES).is_err());
    drop(first);
    assert!(fixture.slot.is_dir());
    assert!(!fixture.slot.join(GENERATION_CONFIG_FILE).exists());

    drop(PendingDirectory::create(&fixture.root).unwrap());
    assert_eq!(pending_slot_count(&fixture), 1);
}

#[test]
fn staging_capacity_is_bounded_and_locked() {
    let fixture = PendingFixture::new();
    let guards = (0..MAX_CONCURRENT_GENERATION_STAGES)
        .map(|_| PendingDirectory::create(&fixture.root).unwrap())
        .collect::<Vec<_>>();
    assert!(PendingDirectory::create(&fixture.root).is_err());
    assert_eq!(
        pending_slot_count(&fixture),
        MAX_CONCURRENT_GENERATION_STAGES
    );
    drop(guards);
    drop(PendingDirectory::create(&fixture.root).unwrap());
}

#[test]
fn unknown_stale_entry_blocks_reuse_and_survives() {
    let fixture = PendingFixture::new();
    fs::create_dir(&fixture.slot).unwrap();
    fs::set_permissions(
        &fixture.slot,
        fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    fs::write(fixture.slot.join(SENTINEL_FILE_NAME), SENTINEL_BYTES).unwrap();

    assert!(PendingDirectory::create(&fixture.root).is_err());
    assert_eq!(
        fs::read(fixture.slot.join(SENTINEL_FILE_NAME)).unwrap(),
        SENTINEL_BYTES
    );
}

#[test]
fn unsafe_stale_slot_is_rejected_without_changing_it() {
    let fixture = PendingFixture::new();
    fs::create_dir(&fixture.slot).unwrap();
    fs::write(fixture.slot.join(SENTINEL_FILE_NAME), SENTINEL_BYTES).unwrap();
    fs::set_permissions(
        &fixture.slot,
        fs::Permissions::from_mode(UNSAFE_STAGING_MODE),
    )
    .unwrap();

    assert!(PendingDirectory::create(&fixture.root).is_err());
    let mode = fs::metadata(&fixture.slot).unwrap().permissions().mode() & PERMISSION_BITS_MASK;
    assert_eq!(mode, UNSAFE_STAGING_MODE);
    assert_eq!(
        fs::read(fixture.slot.join(SENTINEL_FILE_NAME)).unwrap(),
        SENTINEL_BYTES
    );
}

#[test]
fn sealed_crash_leftover_is_reopened_and_recycled() {
    let fixture = PendingFixture::new();
    fs::create_dir(&fixture.slot).unwrap();
    let stale = fixture.slot.join(GENERATION_CONFIG_FILE);
    fs::write(&stale, PENDING_BYTES).unwrap();
    fs::set_permissions(
        &fixture.slot,
        fs::Permissions::from_mode(IMMUTABLE_DIRECTORY_MODE),
    )
    .unwrap();

    let guard = PendingDirectory::create(&fixture.root).unwrap();
    assert!(!stale.exists());
    drop(guard);
    let mode = fs::metadata(&fixture.slot).unwrap().permissions().mode() & PERMISSION_BITS_MASK;
    assert_eq!(mode, PRIVATE_DIRECTORY_MODE);
}

#[test]
fn non_directory_slot_poison_is_preserved() {
    let fixture = PendingFixture::new();
    fs::write(&fixture.slot, SENTINEL_BYTES).unwrap();

    assert!(PendingDirectory::create(&fixture.root).is_err());
    assert_eq!(fs::read(&fixture.slot).unwrap(), SENTINEL_BYTES);
}

#[test]
fn low_level_namespace_failures_are_reported() {
    let missing = entry_name(MISSING_ENTRY_NAME).unwrap();
    assert!(io::inspect_at(INVALID_DESCRIPTOR, &missing).is_err());
    assert!(io::create_directory_at(INVALID_DESCRIPTOR, &missing).is_err());
    let regular = tempfile::tempfile().unwrap();
    assert!(entries::ensure_known(&regular, PathBuf::from(MISSING_ENTRY_NAME).as_path()).is_err());

    let fixture = PendingFixture::new();
    let target = entry_name(TARGET_ENTRY_NAME).unwrap();
    assert!(io::rename_exclusive_at(fixture.root.descriptor(), &missing, &target).is_err());
}

struct PendingFixture {
    base: tempfile::TempDir,
    root: GenerationRoot,
    root_path: PathBuf,
    slot: PathBuf,
}

impl PendingFixture {
    fn new() -> Self {
        let base = tempfile::tempdir().unwrap();
        let root_path = base.path().join(CONFIG_GENERATIONS_DIRECTORY);
        let root = GenerationRoot::ensure(&root_path).unwrap();
        let stage = root_path.join(format!("{PENDING_GENERATION_PREFIX}0"));
        fs::create_dir(&stage).unwrap();
        fs::set_permissions(&stage, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE)).unwrap();
        let slot = root_path.join(format!(
            "{PENDING_GENERATION_PREFIX}0{PENDING_GENERATION_PAYLOAD_SUFFIX}"
        ));
        Self {
            base,
            root,
            root_path,
            slot,
        }
    }
}

fn pending_slot_count(fixture: &PendingFixture) -> usize {
    fs::read_dir(&fixture.root_path)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with(PENDING_GENERATION_PREFIX)
                && !name.ends_with(PENDING_GENERATION_PAYLOAD_SUFFIX)
        })
        .count()
}
