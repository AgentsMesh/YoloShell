pub(crate) fn fail_next_reopen_seal_validation_for_test() {
    macos_reopen_seal_io::fail_next(macos_reopen_seal_io::TestIoOperation::ValidateSeal);
}

pub(crate) fn fail_next_reopen_seal_open_for_test() {
    macos_reopen_seal_io::fail_next(macos_reopen_seal_io::TestIoOperation::OpenSlave);
}

pub(crate) fn fail_reopen_seal_recovery_for_test() -> impl Drop {
    macos_reopen_seal_io::fail_next(macos_reopen_seal_io::TestIoOperation::ValidateSeal);
    macos_reopen_seal_io::fail_repeated_after_next(&[
        macos_reopen_seal_io::TestIoOperation::ChangeMode,
        macos_reopen_seal_io::TestIoOperation::ValidateSeal,
    ])
}
