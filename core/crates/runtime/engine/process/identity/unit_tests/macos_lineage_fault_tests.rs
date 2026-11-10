use super::*;

const QUERY_STEPS: usize = 5;

#[test]
fn every_read_must_succeed_before_a_lineage_can_be_published() {
    for failing_step in 0..QUERY_STEPS {
        for denied in [false, true] {
            let step = Cell::new(0);
            let should_fail = || {
                let current = step.get();
                step.set(current + 1);
                current == failing_step
            };
            let result = capture_with(
                CHILD_PID,
                || reply(short(PARENT_PID), should_fail(), denied),
                || {
                    reply(
                        unique(CHILD_UNIQUE_ID, PARENT_UNIQUE_ID),
                        should_fail(),
                        denied,
                    )
                },
            );
            if denied {
                assert_eq!(
                    result
                        .unwrap_err()
                        .downcast_ref::<std::io::Error>()
                        .unwrap()
                        .raw_os_error(),
                    Some(libc::EPERM)
                );
            } else {
                assert!(result.unwrap().is_none());
            }
            assert_eq!(step.get(), failing_step + 1);
        }
    }
}

#[test]
fn wrong_pid_and_missing_unique_id_never_form_a_boundary() {
    for (pid, child_id, parent_id) in [
        (PARENT_PID, CHILD_UNIQUE_ID, PARENT_UNIQUE_ID),
        (CHILD_PID, 0, PARENT_UNIQUE_ID),
        (CHILD_PID, CHILD_UNIQUE_ID, 0),
    ] {
        assert!(
            capture_with(
                pid,
                || Ok(Some(short(PARENT_PID))),
                || { Ok(Some(unique(child_id, parent_id))) }
            )
            .unwrap_err()
            .to_string()
            .contains(INVALID_IDENTITY_DETAIL)
        );
    }
}

fn reply<T>(value: T, fail: bool, denied: bool) -> Result<Option<T>> {
    if !fail {
        return Ok(Some(value));
    }
    if denied {
        return Err(std::io::Error::from_raw_os_error(libc::EPERM).into());
    }
    Ok(None)
}
