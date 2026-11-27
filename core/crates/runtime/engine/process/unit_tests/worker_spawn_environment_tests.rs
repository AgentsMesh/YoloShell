use super::*;

#[test]
fn command_changes_override_and_remove_inherited_values() {
    let environment = environment_vector_from(
        [
            ("ROLE".into(), "holder".into()),
            ("REMOVE".into(), "secret".into()),
        ],
        &[
            ("ROLE".into(), Some("worker".into())),
            ("REMOVE".into(), None),
            ("ABSENT".into(), None),
        ],
    )
    .unwrap();
    assert_eq!(
        environment._values,
        vec![CString::new("ROLE=worker").unwrap()]
    );
    assert!(environment.pointers.last().unwrap().is_null());
}

#[test]
fn new_empty_and_inherited_values_remain_distinct() {
    let environment = environment_vector_from(
        [("KEEP".into(), "original".into())],
        &[
            ("EMPTY".into(), Some("".into())),
            ("NEW".into(), Some("a=b".into())),
        ],
    )
    .unwrap();
    let values = environment
        ._values
        .iter()
        .map(|value| value.to_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values, ["EMPTY=", "KEEP=original", "NEW=a=b"]);
}

#[test]
fn malformed_environment_cannot_reach_posix_spawn() {
    for (key, value) in [
        ("", "value"),
        ("a=b", "value"),
        ("a\0", "value"),
        ("KEY", "v\0"),
    ] {
        assert!(environment_vector_from([], &[(key.into(), Some(value.into()))]).is_err());
    }
}
