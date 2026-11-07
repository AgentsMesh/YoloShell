use super::{constants::*, fixture::*};
use yolo_hub_remote_setup_protocol::*;

#[test]
fn credentials_are_bounded_redacted_and_cannot_enter_inspection_or_persisted_status() {
    assert!(PrivateCredentials::new(Vec::new()).is_err());
    assert!(PrivateCredentials::new(vec![0; MAX_PRIVATE_CREDENTIAL_BYTES + 1]).is_err());
    let credentials = PrivateCredentials::new(CREDENTIAL_BYTES.to_vec()).unwrap();
    assert!(!format!("{credentials:?}").contains(std::str::from_utf8(CREDENTIAL_BYTES).unwrap()));
    let bytes = serde_json::to_vec(&credentials).unwrap();
    assert_eq!(
        serde_json::from_slice::<PrivateCredentials>(&bytes).unwrap(),
        credentials
    );
    let request = request();
    for action in [
        SetupAction::Start,
        SetupAction::Continue,
        SetupAction::Inspect,
        SetupAction::Poll,
    ] {
        let call = RemoteSetupCall {
            action,
            request: request.clone(),
            credentials: Some(credentials.clone()),
        };
        assert_eq!(
            call.validate().is_ok(),
            matches!(action, SetupAction::Start | SetupAction::Continue)
        );
        assert_eq!(
            call.mutates(),
            matches!(action, SetupAction::Start | SetupAction::Continue)
        );
    }
    let status = RemoteSetupStatus {
        request,
        outcome: RemoteSetupOutcome::Unknown {},
    };
    let wire = serde_json::to_string(&status).unwrap();
    assert!(!wire.contains(std::str::from_utf8(CREDENTIAL_BYTES).unwrap()));
    assert_eq!(&*credentials.into_bytes(), CREDENTIAL_BYTES);
}

#[test]
fn statuses_bind_full_request_and_linked_result_requires_applied_matching_local_configuration() {
    let request = request();
    let provision = ready(&request);
    let mut configuration = baseline(&request);
    let change = plan_link(&configuration, &request, &provision)
        .unwrap()
        .unwrap();
    configuration.state.configuration = change.configuration;
    configuration.state.revision += 1;
    configuration.state.last_change = change.request_id;
    let status = RemoteSetupStatus {
        request: request.clone(),
        outcome: RemoteSetupOutcome::Linked {
            provision,
            configuration,
        },
    };
    status.validate_for(&request).unwrap();
    assert!(status.validate_for(&super::fixture::request()).is_err());
    let mut changed = status.clone();
    if let RemoteSetupOutcome::Linked { configuration, .. } = &mut changed.outcome {
        configuration.applied = false;
    }
    assert!(changed.validate_for(&request).is_err());
    let mut wire = serde_json::to_value(&status).unwrap();
    wire[EXTRA_FIELD] = true.into();
    assert!(serde_json::from_value::<RemoteSetupStatus>(wire).is_err());
    let mut changed = request;
    changed.expected_configuration_revision = INVALID_REVISION;
    assert!(changed.validate().is_err());
}
