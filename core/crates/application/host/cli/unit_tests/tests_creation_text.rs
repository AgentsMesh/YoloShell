#[test]
fn creation_authorization_accepts_uuid_text_and_rejects_malformed_input() {
    let capability = uuid::Uuid::new_v4();
    let parse = || parsed_creation_authorization(cli::CLEANUP_ARCHIVE_COMMAND);
    let readable = parse()
        .read_checked::<ctl_args::CleanupAuthority>(format!("{capability}\n").as_bytes(), false)
        .unwrap()
        .validate()
        .unwrap();
    assert_eq!(readable.creation_capability, capability);
    let padded = format!("\n  {capability}\t\n");
    assert!(
        parse()
            .read_checked::<ctl_args::CleanupAuthority>(padded.as_bytes(), false)
            .is_ok()
    );
    for malformed in [
        String::new(),
        String::from("not-a-uuid"),
        format!("{capability} {}", uuid::Uuid::new_v4()),
    ] {
        assert!(
            parse()
                .read_checked::<ctl_args::CleanupAuthority>(malformed.as_bytes(), false)
                .is_err(),
            "{malformed:?}"
        );
    }
    let nil = format!("{}\n", uuid::Uuid::nil());
    assert!(
        parse()
            .read_checked::<ctl_args::CleanupAuthority>(nil.as_bytes(), false)
            .unwrap()
            .validate()
            .is_err()
    );
    assert!(
        parse()
            .read_checked::<ctl_args::CleanupAuthority>([0u8, 1, 2, 3].as_slice(), false)
            .is_err()
    );
}

#[test]
fn finalize_authorization_text_keeps_token_before_capability() {
    let token = uuid::Uuid::new_v4();
    let capability = uuid::Uuid::new_v4();
    let text = format!("{token}\n{capability}\n");
    let authority = parsed_creation_authorization(cli::FINALIZE_CREATION_COMMAND)
        .read_checked::<ctl_args::FinalizeAuthority>(text.as_bytes(), false)
        .unwrap()
        .validate()
        .unwrap();
    assert_eq!(authority.creation_token, token);
    assert_eq!(authority.creation_capability, capability);
    let missing = format!("{token}\n");
    assert!(
        parsed_creation_authorization(cli::FINALIZE_CREATION_COMMAND)
            .read_checked::<ctl_args::FinalizeAuthority>(missing.as_bytes(), false)
            .is_err()
    );
}
