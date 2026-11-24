use super::*;

const ACCOUNT_SHELL: &[u8] = b"/bin/account-shell";
const INVALID_SYSCONF_RESULT: libc::c_long = -1;
const LOOKUP_FAILURE_STATUS: libc::c_int = libc::EIO;
const TEST_USER_ID: libc::uid_t = 501;

#[test]
fn account_buffer_hint_is_bounded_and_has_a_deterministic_default() {
    assert_eq!(
        normalize_initial_buffer_bytes(INVALID_SYSCONF_RESULT),
        ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES
    );
    assert_eq!(
        normalize_initial_buffer_bytes(libc::c_long::default()),
        ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES
    );
    assert_eq!(
        normalize_initial_buffer_bytes(ACCOUNT_RECORD_BUFFER_MIN_BYTES as libc::c_long),
        ACCOUNT_RECORD_BUFFER_MIN_BYTES
    );
    assert_eq!(
        normalize_initial_buffer_bytes(libc::c_long::MAX),
        ACCOUNT_RECORD_BUFFER_MAX_BYTES
    );
}

#[test]
fn account_lookup_retries_buffer_and_interrupt_boundaries() {
    let mut first = true;
    let shell = login_shell_with(
        TEST_USER_ID,
        ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES,
        |user_id, record, buffer, result| {
            assert_eq!(user_id, TEST_USER_ID);
            if std::mem::take(&mut first) {
                return libc::ERANGE;
            }
            provide_shell(record, buffer, result);
            ACCOUNT_LOOKUP_SUCCESS
        },
    );
    assert_eq!(shell, Some(OsString::from_vec(ACCOUNT_SHELL.to_vec())));
    assert!(!first);

    let mut interruptions = Vec::new();
    assert_eq!(
        login_shell_with(
            TEST_USER_ID,
            ACCOUNT_RECORD_BUFFER_MAX_BYTES,
            |_, _, buffer, _| {
                interruptions.push(buffer.len());
                libc::EINTR
            }
        ),
        None
    );
    assert_eq!(
        interruptions.len(),
        ACCOUNT_LOOKUP_MAX_INTERRUPT_RETRIES + RETRY_COUNT_STEP
    );
}

#[test]
fn account_lookup_rejects_failed_or_incomplete_records() {
    assert_eq!(
        login_shell_with(
            TEST_USER_ID,
            ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES,
            |_, _, _, _| { LOOKUP_FAILURE_STATUS }
        ),
        None
    );
    assert_eq!(
        login_shell_with(
            TEST_USER_ID,
            ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES,
            |_, _, _, _| ACCOUNT_LOOKUP_SUCCESS,
        ),
        None
    );
    assert_eq!(
        login_shell_with(
            TEST_USER_ID,
            ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES,
            |_, record, buffer, result| {
                provide_shell(record, buffer, result);
                *result = ptr::NonNull::<libc::passwd>::dangling().as_ptr();
                ACCOUNT_LOOKUP_SUCCESS
            },
        ),
        None
    );
    assert_eq!(
        login_shell_with(
            TEST_USER_ID,
            ACCOUNT_RECORD_BUFFER_MAX_BYTES,
            |_, _, _, _| libc::ERANGE,
        ),
        None
    );
}

#[test]
fn account_lookup_rejects_unbounded_shell_pointers() {
    assert_eq!(
        login_shell_with(
            TEST_USER_ID,
            ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES,
            |_, record, _, result| {
                *result = ptr::from_mut(record);
                ACCOUNT_LOOKUP_SUCCESS
            },
        ),
        None
    );
    assert_eq!(
        login_shell_with(
            TEST_USER_ID,
            ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES,
            |_, record, buffer, result| {
                buffer.fill(libc::c_char::MAX);
                record.pw_shell = buffer.as_mut_ptr();
                *result = ptr::from_mut(record);
                ACCOUNT_LOOKUP_SUCCESS
            },
        ),
        None
    );
    for after_buffer in [false, true] {
        assert_eq!(
            login_shell_with(
                TEST_USER_ID,
                ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES,
                |_, record, buffer, result| {
                    record.pw_shell = if after_buffer {
                        buffer
                            .as_mut_ptr()
                            .wrapping_add(buffer.len().saturating_add(RETRY_COUNT_STEP))
                    } else {
                        buffer.as_mut_ptr().wrapping_sub(RETRY_COUNT_STEP)
                    };
                    *result = ptr::from_mut(record);
                    ACCOUNT_LOOKUP_SUCCESS
                },
            ),
            None
        );
    }
}

fn provide_shell(
    record: &mut libc::passwd,
    buffer: &mut [libc::c_char],
    result: &mut *mut libc::passwd,
) {
    assert!(ACCOUNT_SHELL.len() < buffer.len());
    for (destination, source) in buffer.iter_mut().zip(ACCOUNT_SHELL) {
        *destination = *source as libc::c_char;
    }
    record.pw_shell = buffer.as_mut_ptr();
    *result = ptr::from_mut(record);
}
