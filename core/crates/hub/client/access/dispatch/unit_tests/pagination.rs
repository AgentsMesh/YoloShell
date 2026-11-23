use super::{backend::*, model::*};
use crate::{ScopedDispatcher, constants::MAX_CATALOGUE_SCAN_PAGES};
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, HubSummary, Reply};
use yolo_hub_terminal_protocol::PublicError;

const PAGE_SIZE: usize = 1;

async fn hubs(
    scoped: &ScopedDispatcher,
    after: Option<yolo_hub_identity::HubId>,
) -> Result<(Vec<HubSummary>, Option<yolo_hub_identity::HubId>), PublicError> {
    scoped
        .page(
            after,
            PAGE_SIZE,
            |after| Command::ListHubs { after },
            |reply| {
                if let Reply::Hubs { hubs, next } = reply {
                    Ok((hubs, next))
                } else {
                    Err(PublicError::InvalidPayload)
                }
            },
            |hub| hub.hub_id,
            |hub| hub.hub_id,
        )
        .await
}

#[tokio::test]
async fn hidden_pages_never_supply_a_public_cursor_and_regressions_are_rejected() {
    let (_registry, admission) = authority(&[Right::Observe]);
    let hidden = HubSummary {
        hub_id: hub(HIDDEN),
        reachable: true,
        label: None,
    };
    let allowed = HubSummary {
        hub_id: hub(ORIGIN),
        reachable: true,
        label: None,
    };
    let backend = Scripted::new([
        Reply::Hubs {
            hubs: vec![hidden],
            next: Some(hidden.hub_id),
        },
        Reply::Hubs {
            hubs: vec![allowed],
            next: None,
        },
    ]);
    let scoped = ScopedDispatcher::new(backend.clone(), admission.clone()).unwrap();
    assert_eq!(hubs(&scoped, None).await.unwrap(), (vec![allowed], None));
    assert_eq!(backend.calls.lock().unwrap().len(), 2);
    let backend = Scripted::new([Reply::Hubs {
        hubs: vec![hidden],
        next: None,
    }]);
    let scoped = ScopedDispatcher::new(backend, admission).unwrap();
    assert_eq!(
        hubs(&scoped, Some(allowed.hub_id)).await,
        Err(PublicError::InvalidPayload)
    );
}

#[tokio::test]
async fn malformed_and_remote_errors_propagate_without_exposing_rows() {
    let (_registry, admission) = authority(&[Right::Observe]);
    for (reply, expected) in [
        (Reply::Completed {}, PublicError::InvalidPayload),
        (
            Reply::Error {
                code: PublicError::Unreachable,
            },
            PublicError::Unreachable,
        ),
        (
            Reply::Hubs {
                hubs: vec![],
                next: Some(hub(ORIGIN)),
            },
            PublicError::InvalidPayload,
        ),
    ] {
        let scoped = ScopedDispatcher::new(Scripted::new([reply]), admission.clone()).unwrap();
        assert_eq!(hubs(&scoped, None).await, Err(expected));
    }
}

#[tokio::test]
async fn hidden_catalogue_scans_have_a_hard_bound() {
    let (_registry, admission) = authority(&[Right::Observe]);
    let replies = (1..=MAX_CATALOGUE_SCAN_PAGES).map(|index| {
        let origin = hub((ORIGIN as usize + index) as u128);
        Reply::Hubs {
            hubs: vec![HubSummary {
                hub_id: origin,
                reachable: true,
                label: None,
            }],
            next: Some(origin),
        }
    });
    let backend = Scripted::new(replies);
    let scoped = ScopedDispatcher::new(backend.clone(), admission).unwrap();
    assert_eq!(hubs(&scoped, None).await, Err(PublicError::InvalidPayload));
    assert_eq!(
        backend.calls.lock().unwrap().len(),
        MAX_CATALOGUE_SCAN_PAGES
    );
}
