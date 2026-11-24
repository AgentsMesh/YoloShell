fn run_install(case: InstallCase) {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_NAME);
    write_private(&target, ORIGINAL_BYTES);
    let entry = PrivateFileEntry::ensure(&target).unwrap();
    let expected = entry.open_retained(u64::MAX).unwrap().identity;
    let mut temporary = stage(&entry, StageCase::Valid).unwrap().0;
    let saved = root.path().join(SAVED_NAME);
    match case {
        InstallCase::InstallHookFailure => {
            set_retained_install_hook(|| anyhow::bail!(INSTALL_HOOK_ERROR));
        }
        InstallCase::InstallReplacement => {
            let occupied = target.clone();
            set_retained_install_hook(move || {
                write_private(&occupied, OCCUPANT_BYTES);
                Ok(())
            });
        }
        InstallCase::IsolationReplacement => {
            let occupied = target.clone();
            set_retained_exchange_hook(move || {
                fs::rename(&occupied, &saved)?;
                write_private(&occupied, OCCUPANT_BYTES);
                Ok(())
            });
        }
        InstallCase::RenameFailure => {
            let occupied = target.clone();
            set_retained_rename_hook(move || {
                write_private(&occupied, OCCUPANT_BYTES);
                Ok(())
            });
        }
        InstallCase::StageMutation => {
            let mut staged = temporary.file.try_clone().unwrap();
            set_retained_rename_hook(move || {
                staged.rewind()?;
                staged.write_all(OCCUPANT_BYTES)?;
                staged.sync_all()?;
                Ok(())
            });
        }
        InstallCase::RenameHookFailure => {
            set_retained_rename_hook(|| anyhow::bail!(RENAME_HOOK_ERROR));
        }
        InstallCase::Success | InstallCase::FenceFailure | InstallCase::SecondFenceFailure => {}
    }
    let mut fence_calls = INITIAL_FENCE_CALL_COUNT;
    let result = entry.install_retained(&mut temporary, expected, u64::MAX, || {
        fence_calls = fence_calls
            .checked_add(FENCE_CALL_INCREMENT)
            .expect(FENCE_CALL_OVERFLOW);
        if matches!(case, InstallCase::FenceFailure)
            || matches!(case, InstallCase::SecondFenceFailure) && fence_calls == SECOND_FENCE_CALL
        {
            anyhow::bail!(FENCE_ERROR);
        }
        Ok(())
    });
    assert_eq!(result.is_ok(), matches!(case, InstallCase::Success));
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
