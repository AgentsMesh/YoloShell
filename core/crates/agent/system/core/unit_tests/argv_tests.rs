use super::*;

#[test]
fn nul_fields_preserve_argument_boundaries() {
    assert_eq!(
        parse_nul_strings(b"node\0grep @openai/codex\0\0", usize::MAX),
        ["node", "grep @openai/codex"]
    );
    assert_eq!(parse_nul_strings(b"one\0two\0three\0", 2), ["one", "two"]);
    assert_eq!(parse_nul_strings(b"\0\0", usize::MAX), Vec::<String>::new());
}

#[cfg(target_os = "macos")]
#[test]
fn macos_procargs_preserves_argv_and_selects_only_allowed_environment() {
    let bytes = procargs(
        &[b"node", b"", b"entry.js"],
        &[
            b"HOME=/Users/test",
            b"CODEX_HOME=/var/codex",
            b"AUTH_TOKEN=never-retain-this",
        ],
    );
    let (argv, environment) = parse_macos_procargs(&bytes, selected_keys()).unwrap();
    assert_eq!(argv, ["node", "", "entry.js"]);
    assert_eq!(environment.len(), 2);
    assert_eq!(
        environment[super::super::constants::HOME_ENV],
        "/Users/test"
    );
    assert_eq!(
        environment[crate::provider_fixture::PRIMARY_DATA_HOME_ENV],
        "/var/codex"
    );
    assert!(!environment.contains_key("AUTH_TOKEN"));
}

#[cfg(target_os = "macos")]
#[test]
fn macos_procargs_rejects_malformed_boundaries() {
    assert!(parse_macos_procargs(&[], selected_keys()).is_err());
    assert!(parse_macos_procargs(&(-1_i32).to_ne_bytes(), selected_keys()).is_err());
    let mut missing_terminator = 1_i32.to_ne_bytes().to_vec();
    missing_terminator.extend_from_slice(b"/opt/node");
    assert!(parse_macos_procargs(&missing_terminator, selected_keys()).is_err());

    let mut missing_argument = procargs(&[b"codex"], &[]);
    missing_argument[..std::mem::size_of::<i32>()].copy_from_slice(&2_i32.to_ne_bytes());
    assert!(parse_macos_procargs(&missing_argument, selected_keys()).is_err());

    let mut truncated_environment = procargs(&[b"codex"], &[b"HOME=/Users/test"]);
    truncated_environment.pop();
    assert!(parse_macos_procargs(&truncated_environment, selected_keys()).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn macos_procargs_rejects_ambiguous_or_invalid_selected_environment() {
    let duplicate = procargs(&[b"codex"], &[b"CODEX_HOME=/first", b"CODEX_HOME=/second"]);
    assert!(parse_macos_procargs(&duplicate, selected_keys()).is_err());

    let invalid_utf8 = procargs(&[b"codex"], &[b"CODEX_HOME=/tmp/\xff"]);
    assert!(parse_macos_procargs(&invalid_utf8, selected_keys()).is_err());

    let mut ignored_long = b"AUTH_TOKEN=".to_vec();
    ignored_long.extend(std::iter::repeat_n(
        b'x',
        super::super::constants::MAX_PROCESS_ENVIRONMENT_VALUE_BYTES + 1,
    ));
    let ignored_fields = procargs(
        &[b"codex"],
        &[
            &ignored_long,
            b"AUTH_TOKEN",
            b"AUTH_TOKEN=\xff",
            b"HOME=/srv/project",
        ],
    );
    let (_, environment) = parse_macos_procargs(&ignored_fields, selected_keys()).unwrap();
    assert_eq!(
        environment[super::super::constants::HOME_ENV],
        "/srv/project"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_procargs_enforces_environment_resource_limits() {
    use super::super::constants::{
        MAX_PROCESS_ENVIRONMENT_BYTES, MAX_PROCESS_ENVIRONMENT_FIELDS,
        MAX_PROCESS_ENVIRONMENT_KEYS, MAX_PROCESS_ENVIRONMENT_VALUE_BYTES,
    };
    let mut oversized = b"CODEX_HOME=".to_vec();
    oversized.extend(std::iter::repeat_n(
        b'x',
        MAX_PROCESS_ENVIRONMENT_VALUE_BYTES + 1,
    ));
    let bytes = procargs(&[b"codex"], &[&oversized]);
    assert!(parse_macos_procargs(&bytes, selected_keys()).is_err());

    let fields = vec![b"IGNORED=value".as_slice(); MAX_PROCESS_ENVIRONMENT_FIELDS + 1];
    let bytes = procargs(&[b"codex"], &fields);
    assert!(parse_macos_procargs(&bytes, selected_keys()).is_err());

    let names: Vec<_> = (0..MAX_PROCESS_ENVIRONMENT_KEYS)
        .map(|index| format!("SELECTED_{index}"))
        .collect();
    let value = "x".repeat(MAX_PROCESS_ENVIRONMENT_VALUE_BYTES);
    let selected: Vec<_> = names
        .iter()
        .map(|name| format!("{name}={value}").into_bytes())
        .collect();
    let selected_refs: Vec<_> = selected.iter().map(Vec::as_slice).collect();
    let key_refs: Vec<_> = names.iter().map(String::as_str).collect();
    assert!(selected.iter().map(Vec::len).sum::<usize>() > MAX_PROCESS_ENVIRONMENT_BYTES);
    let bytes = procargs(&[b"codex"], &selected_refs);
    assert!(parse_macos_procargs(&bytes, &key_refs).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn process_argv_observes_current_process_and_rejects_oversized_pid() {
    use super::super::constants::MAX_PROCESS_ENVIRONMENT_KEYS;
    let snapshot = process_argv_bounded(std::process::id(), usize::MAX, selected_keys()).unwrap();
    assert!(!snapshot.values.is_empty());
    assert!(snapshot.bytes > 0);
    assert!(
        snapshot
            .environment
            .keys()
            .all(|key| selected_keys().contains(&key.as_str()))
    );
    assert!(process_argv_bounded(std::process::id(), 0, selected_keys()).is_err());
    assert!(process_argv_bounded(0, usize::MAX, selected_keys()).is_err());
    assert!(process_argv_bounded(u32::MAX, usize::MAX, selected_keys()).is_err());
    let too_many_keys = vec![super::super::constants::HOME_ENV; MAX_PROCESS_ENVIRONMENT_KEYS + 1];
    assert!(process_argv_bounded(std::process::id(), usize::MAX, &too_many_keys).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn sysctl_status_preserves_the_operation_context() {
    ensure_sysctl_success(0, "successful operation").unwrap();
    let error = ensure_sysctl_success(-1, "injected sysctl failure").unwrap_err();
    assert!(error.to_string().contains("injected sysctl failure"));
}

#[cfg(target_os = "macos")]
fn selected_keys() -> &'static [&'static str] {
    &[
        super::super::constants::HOME_ENV,
        crate::provider_fixture::PRIMARY_DATA_HOME_ENV,
        crate::provider_fixture::SECONDARY_DATA_HOME_ENV,
    ]
}

#[cfg(target_os = "macos")]
fn procargs(arguments: &[&[u8]], environment: &[&[u8]]) -> Vec<u8> {
    use super::super::constants::NUL_BYTE;
    let mut bytes = i32::try_from(arguments.len())
        .unwrap()
        .to_ne_bytes()
        .to_vec();
    bytes.extend_from_slice(b"/opt/yolo-test");
    bytes.extend([NUL_BYTE, NUL_BYTE]);
    for field in arguments.iter().chain(environment) {
        bytes.extend_from_slice(field);
        bytes.push(NUL_BYTE);
    }
    bytes
}
