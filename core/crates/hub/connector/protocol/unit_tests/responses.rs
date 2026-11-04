use crate::{constants::*, fixture::Fixture};
use yolo_hub_agent_protocol::ObservedProvider;
use yolo_hub_connector_protocol::*;
use yolo_hub_identity::{ConnectorId, HubId, RuntimeEpoch};

#[test]
fn each_reply_family_matches_only_its_request_and_error_replies_still_require_exact_identity() {
    let fixture = Fixture::new();
    for (request, own) in fixture.samples() {
        for (_, candidate) in fixture.samples() {
            let same = std::mem::discriminant(&candidate) == std::mem::discriminant(&own);
            assert_eq!(
                Response::new(&request, candidate)
                    .validate_for(&request)
                    .is_ok(),
                same
            );
        }
        let good = Response::new(&request, own);
        assert_eq!(
            serde_json::from_slice::<Response>(&serde_json::to_vec(&good).unwrap()).unwrap(),
            good
        );
        let error = Response::new(
            &request,
            Reply::Error {
                code: ConnectorError::Unauthorized,
            },
        );
        error.validate_for(&request).unwrap();
        let mut changed = error.clone();
        changed.request_id = uuid::Uuid::new_v4();
        assert_eq!(
            changed.validate_for(&request),
            Err(ConnectorError::InvalidRequest)
        );
        let mut changed = error.clone();
        changed.hub = HubId::new();
        assert_eq!(
            changed.validate_for(&request),
            Err(ConnectorError::InvalidRequest)
        );
        let mut changed = error;
        changed.version = 0;
        assert_eq!(
            changed.validate_for(&request),
            Err(ConnectorError::UnsupportedSchema)
        );
    }
}

#[test]
fn registration_echoes_whole_owner_authorized_provider_descriptor_and_terminal_scope() {
    let fixture = Fixture::new();
    let request = Request::new(
        fixture.hub,
        Command::Register {
            scope: fixture.enrollment.scope.clone(),
        },
    )
    .unwrap();
    let mutations: &[fn(&mut Enrollment)] = &[
        |e| e.scope.terminal.runtime_epoch = RuntimeEpoch::new(),
        |e| {
            e.scope.provider = ObservedProvider::new(
                PROVIDER_KIND.into(),
                Some(DIFFERENT_VERSION.into()),
                Some(PROVIDER_REFERENCE.into()),
            )
            .unwrap()
        },
        |e| {
            e.scope.provider =
                ObservedProvider::new(PROVIDER_KIND.into(), Some(PROVIDER_VERSION.into()), None)
                    .unwrap()
        },
        |e| e.agent.hub_id = HubId::new(),
    ];
    for mutate in mutations {
        let mut changed = fixture.enrollment.clone();
        mutate(&mut changed);
        assert!(
            Response::new(
                &request,
                Reply::Enrolled {
                    enrollment: changed
                }
            )
            .validate_for(&request)
            .is_err()
        );
    }
}

#[test]
fn activation_and_revocation_cannot_replace_connector_or_revision() {
    let fixture = Fixture::new();
    let activate = Request::new(
        fixture.hub,
        Command::Activate {
            connector_id: fixture.enrollment.connector_id,
            credential: fixture.enrollment.credential,
            instance_id: yolo_hub_identity::ConnectorInstanceId::new(),
        },
    )
    .unwrap();
    let mut wrong = fixture.activation;
    wrong.connector_id = ConnectorId::new();
    assert_eq!(
        Response::new(&activate, Reply::Activated { activation: wrong }).validate_for(&activate),
        Err(ConnectorError::InvalidRequest)
    );
    let revoke = Request::new(
        fixture.hub,
        Command::Revoke {
            connector_id: fixture.enrollment.connector_id,
            expected_revision: fixture.enrollment.revision,
        },
    )
    .unwrap();
    let mut summary = fixture.summary(EnrollmentStatus::Revoked);
    summary.revision += REVISION_INCREMENT;
    Response::new(
        &revoke,
        Reply::Revoked {
            enrollment: summary.clone(),
        },
    )
    .validate_for(&revoke)
    .unwrap();
    for mutate in [
        (|s: &mut EnrollmentSummary| s.connector_id = ConnectorId::new())
            as fn(&mut EnrollmentSummary),
        |s| s.status = EnrollmentStatus::Enabled,
        |s| s.revision -= REVISION_INCREMENT,
    ] {
        let mut wrong = summary.clone();
        mutate(&mut wrong);
        assert!(
            Response::new(&revoke, Reply::Revoked { enrollment: wrong })
                .validate_for(&revoke)
                .is_err()
        );
    }
}
