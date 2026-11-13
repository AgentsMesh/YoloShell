use super::*;

#[tokio::test]
async fn serve_requires_revision_before_touching_the_state_directory() {
    let fixture = support::RuntimeFixture::new();
    let state_dir = fixture.options.broker_state.clone();
    let error = run(Cli {
        command: Command::Serve(server_options::from_request(&fixture.options)),
    })
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), crate::error_constants::REVISION_REQUIRED);
    assert!(!state_dir.exists());
}

#[tokio::test]
async fn incomplete_profile_authority_cannot_create_broker_state() {
    let fixture = support::RuntimeFixture::new();
    let mut options = server_options::from_request(&fixture.options);
    for expected in [
        crate::error_constants::DOCUMENT_REQUIRED,
        crate::error_constants::LOCK_REQUIRED,
        crate::error_constants::PROFILE_REQUIRED,
    ] {
        let error = run(Cli {
            command: Command::Start(options.clone()),
        })
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), expected);
        assert!(!fixture.options.broker_state.exists());
        match expected {
            crate::error_constants::DOCUMENT_REQUIRED => {
                options.authority_document = Some(options.config.clone());
            }
            crate::error_constants::LOCK_REQUIRED => {
                options.authority_lock = Some(options.config.clone());
            }
            _ => {}
        }
    }
}
