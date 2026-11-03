use super::*;

#[tokio::test]
async fn creation_query_observes_each_phase_without_materializing_a_runtime() {
    for phase in [
        CreationPhase::Reserved,
        CreationPhase::Launching,
        CreationPhase::SpawnFailed,
        CreationPhase::Ready,
        CreationPhase::Terminated,
    ] {
        let mut fixture = QueryFixture::new();
        fixture.phase(phase);
        let expected = if phase == CreationPhase::Ready {
            CreationQueryOutcome::Ready {
                creation: fixture.binding.result,
            }
        } else {
            CreationQueryOutcome::Pending {}
        };
        assert_eq!(fixture.outcome().await, expected);
        assert!(
            !fixture
                .host
                .paths
                .session_dir(fixture.binding.result.terminal_id)
                .exists()
        );
    }
    assert!(!yolo_host_change_feed::mutates(
        method::QUERY_CREATION
    ));
}

#[tokio::test]
async fn missing_is_only_an_observation_before_a_later_reservation() {
    let mut fixture = QueryFixture::new();
    let mut later = fixture.request.clone();
    later.creation_token = Uuid::new_v4();
    assert_eq!(
        fixture.observe(&later).await.unwrap().outcome,
        CreationQueryOutcome::Missing {}
    );
    fixture.binding = fixture.reserve(&later);
    fixture.request = later;
    assert_eq!(fixture.outcome().await, CreationQueryOutcome::Pending {});
    fixture.phase(CreationPhase::Ready);
    assert_eq!(
        fixture.outcome().await,
        CreationQueryOutcome::Ready {
            creation: fixture.binding.result
        }
    );
}

#[tokio::test]
async fn token_does_not_authorize_a_different_host_creation_specification() {
    let fixture = QueryFixture::new();
    let changes: &[fn(&mut CreateTerminal)] = &[
        |query| query.name = Some(OTHER_NAME.into()),
        |query| query.name = None,
        |query| query.cwd = OTHER_CWD.into(),
        |query| query.cols = None,
        |query| query.rows = None,
    ];
    for change in changes {
        let mut query = fixture.request.clone();
        change(&mut query);
        let error = fixture.observe(&query).await.unwrap_err();
        assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    }
    let mut invalid = fixture.request.clone();
    invalid.creation_token = Uuid::nil();
    assert_eq!(
        fixture.observe(&invalid).await.unwrap_err().code,
        crate::protocol::error_code::INVALID_REQUEST
    );
}

#[tokio::test]
async fn query_fingerprint_uses_original_cwd_without_resolving_it_again() {
    let fixture = QueryFixture::new();
    let mut original = fixture.request.clone();
    let cwd = fixture.directory.path().join(QUERY_CWD_DIRECTORY);
    std::fs::create_dir(&cwd).unwrap();
    original.cwd = cwd.join(QUERY_RAW_CWD_COMPONENT);
    let spec =
        CreationSpec::resolve(&original, CreationSpec::fingerprint(&original).unwrap()).unwrap();
    let mut ledger = lock_recover(&fixture.host.creation_capabilities);
    let mut candidate = ledger.clone();
    let binding = candidate
        .bindings
        .iter_mut()
        .find(|entry| entry.result == fixture.binding.result)
        .unwrap();
    binding.spec = spec;
    ledger.install(&fixture.ledger_path, candidate).unwrap();
    drop(ledger);
    std::fs::remove_dir(cwd).unwrap();
    assert_eq!(
        fixture.observe(&original).await.unwrap().outcome,
        CreationQueryOutcome::Pending {}
    );
    assert_eq!(
        fixture.observe(&fixture.request).await.unwrap_err().code,
        crate::protocol::error_code::CONFLICT
    );
}
