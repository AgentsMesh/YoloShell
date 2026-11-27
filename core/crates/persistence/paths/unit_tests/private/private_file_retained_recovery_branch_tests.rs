const RECOVERY_ROLLED_BACK_DETAIL: &str = "recovery was rolled back";
const RECOVERY_PRESERVED_DETAIL: &str = "recovery was preserved at target";
const RECYCLE_ROLLED_BACK_DETAIL: &str = "recycle was rolled back";
const RECYCLE_PRESERVED_DETAIL: &str = "recycle was preserved";
const SAVED_RECOVERY_NAME: &str = "saved-retained-recovery";

#[test]
fn isolated_verification_rejects_a_disappeared_name() {
    let fixture = Fixture::new();
    let expected = fixture.prepare_isolated();
    fs::remove_file(&fixture.isolated).unwrap();

    assert!(
        fixture
            .entry
            .verify_isolated_retained(expected, u64::MAX)
            .is_err()
    );
}

#[test]
fn invalid_moved_content_is_rolled_back_for_every_recovery_operation() {
    for operation in [RecoveryOperation::Restore, RecoveryOperation::Recycle] {
        let fixture = Fixture::new();
        let expected = fixture.prepare_isolated();
        let destination = operation.destination(&fixture);
        let moved = destination.clone();

        let error = operation
            .run_with_hook(&fixture.entry, expected, move || {
                fs::write(&moved, REPLACEMENT_BYTES).unwrap();
            })
            .unwrap_err();

        assert!(error.to_string().contains(operation.rolled_back_detail()));
        assert_eq!(fs::read(&fixture.isolated).unwrap(), REPLACEMENT_BYTES);
        assert!(!destination.exists());
    }
}

#[test]
fn missing_moved_name_preserves_competing_isolated_occupant() {
    for operation in [RecoveryOperation::Restore, RecoveryOperation::Recycle] {
        let fixture = Fixture::new();
        let expected = fixture.prepare_isolated();
        let destination = operation.destination(&fixture);
        let moved = destination.clone();
        let isolated = fixture.isolated.clone();
        let saved = fixture.root.path().join(SAVED_RECOVERY_NAME);
        let saved_for_hook = saved.clone();

        let error = operation
            .run_with_hook(&fixture.entry, expected, move || {
                fs::rename(&moved, &saved_for_hook).unwrap();
                write_private(&isolated, OCCUPANT_BYTES, PRIVATE_FILE_MODE);
            })
            .unwrap_err();

        assert!(error.to_string().contains(operation.preserved_detail()));
        assert_eq!(fs::read(&fixture.isolated).unwrap(), OCCUPANT_BYTES);
        assert_eq!(fs::read(saved).unwrap(), ISOLATED_BYTES);
        assert!(!destination.exists());
    }
}

impl RecoveryOperation {
    fn run_with_hook<F>(
        self,
        entry: &PrivateFileEntry,
        expected: FileIdentity,
        hook: F,
    ) -> anyhow::Result<()>
    where
        F: FnMut(),
    {
        match self {
            Self::Restore => entry.restore_isolated_retained_with_hook(expected, u64::MAX, hook),
            Self::Recycle => entry.recycle_isolated_retained_with_hook(expected, u64::MAX, hook),
        }
    }

    fn rolled_back_detail(self) -> &'static str {
        match self {
            Self::Restore => RECOVERY_ROLLED_BACK_DETAIL,
            Self::Recycle => RECYCLE_ROLLED_BACK_DETAIL,
        }
    }

    fn preserved_detail(self) -> &'static str {
        match self {
            Self::Restore => RECOVERY_PRESERVED_DETAIL,
            Self::Recycle => RECYCLE_PRESERVED_DETAIL,
        }
    }
}

impl Fixture {
    fn prepare_isolated(&self) -> FileIdentity {
        self.write_isolated(ISOLATED_BYTES, PRIVATE_FILE_MODE);
        let opened = self
            .entry
            .open_isolated_retained(u64::MAX)
            .unwrap()
            .unwrap();
        opened.identity
    }
}

fn named_path(target: &Path, name: &std::ffi::CStr) -> PathBuf {
    target.with_file_name(OsStr::from_bytes(name.to_bytes()))
}
