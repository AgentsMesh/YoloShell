const INJECTED_ROLLBACK_ERROR: &str = "injected private publish rollback";
const ROLLED_BACK_DETAIL: &str = "exchange was rolled back";
const TARGET_REPLACED_DETAIL: &str = "target was already replaced";

#[test]
fn proven_exchange_rollback_restores_both_names() {
    let fixture = ExchangedFixture::new();

    let error = fixture.rollback(|| {}).unwrap_err();

    assert!(error.to_string().contains(ROLLED_BACK_DETAIL));
    assert_eq!(fs::read(&fixture.target).unwrap(), ORIGINAL_BYTES);
    assert_eq!(fs::read(&fixture.stage).unwrap(), STAGED_BYTES);
}

#[test]
fn post_rollback_target_replacement_is_preserved() {
    let fixture = ExchangedFixture::new();
    let target = fixture.target.clone();
    let saved = fixture.saved.clone();

    let error = fixture
        .rollback(move || {
            fs::rename(&target, &saved).unwrap();
            write_private(&target, REPLACEMENT_BYTES, PRIVATE_FILE_MODE);
        })
        .unwrap_err();

    assert!(error.to_string().contains(TARGET_REPLACED_DETAIL));
    assert_eq!(fs::read(&fixture.target).unwrap(), REPLACEMENT_BYTES);
    assert_eq!(fs::read(&fixture.saved).unwrap(), ORIGINAL_BYTES);
    assert_eq!(fs::read(&fixture.stage).unwrap(), STAGED_BYTES);
}

struct ExchangedFixture {
    _root: tempfile::TempDir,
    entry: PrivateFileEntry,
    previous: FileIdentity,
    saved: PathBuf,
    stage: PathBuf,
    target: PathBuf,
    temporary: TemporaryPrivateFile,
}

impl ExchangedFixture {
    fn new() -> Self {
        let root = private_root();
        let target = root.path().join(TARGET_NAME);
        write_private(&target, ORIGINAL_BYTES, PRIVATE_FILE_MODE);
        let entry = PrivateFileEntry::ensure(&target).unwrap();
        let temporary = prepared_temporary(&entry, &target);
        let previous = temporary.previous.as_ref().unwrap().identity;
        let stage = named_path(&target, &temporary.name);
        let saved = root.path().join(SAVED_STAGED_NAME);
        exchange_at(entry.directory.descriptor(), &temporary.name, &entry.name).unwrap();
        entry.directory.sync().unwrap();
        Self {
            _root: root,
            entry,
            previous,
            saved,
            stage,
            target,
            temporary,
        }
    }

    fn rollback(self: &Self, after_rollback: impl FnMut()) -> anyhow::Result<()> {
        self.entry.rollback_exchange(
            &self.temporary,
            self.previous,
            u64::MAX,
            anyhow::anyhow!(INJECTED_ROLLBACK_ERROR),
            after_rollback,
        )
    }
}
