use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use serde_json::Value;
use yolo_hub_rpc_protocol::{
    Command, HUB_ID_FIELD, HUB_REACHABLE_FIELD, HubSummary, MAX_CATALOG_PAGE_HUBS, Reply, Request,
    Response,
};
use yolo_hub_terminal_protocol::PublicError;

fn summary(value: u128, reachable: bool) -> HubSummary {
    HubSummary {
        hub_id: hub(value),
        reachable,
        label: None,
    }
}

#[test]
fn hub_summary_wire_is_strict_and_reachability_does_not_change_identity() {
    for reachable in [false, true] {
        let original = summary(REQUEST_ID, reachable);
        let wire = serde_json::to_value(&original).unwrap();
        assert_eq!(
            serde_json::from_value::<HubSummary>(wire.clone()).unwrap(),
            original
        );
        let mut unknown = wire.clone();
        unknown[UNKNOWN_FIELD] = Value::Bool(true);
        assert!(serde_json::from_value::<HubSummary>(unknown).is_err());
        for field in [HUB_ID_FIELD, HUB_REACHABLE_FIELD] {
            let mut missing = wire.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<HubSummary>(missing).is_err());
        }
        let mut invalid = wire.clone();
        invalid[HUB_REACHABLE_FIELD] = Value::String(UNKNOWN_TEXT.into());
        assert!(serde_json::from_value::<HubSummary>(invalid).is_err());
        for replacement in [
            id(REQUEST_ID).simple().to_string(),
            id(REQUEST_ID).to_string().to_uppercase(),
            uuid::Uuid::nil().to_string(),
        ] {
            let mut invalid = wire.clone();
            invalid[HUB_ID_FIELD] = Value::String(replacement);
            assert!(serde_json::from_value::<HubSummary>(invalid).is_err());
        }
    }
}

#[test]
fn list_hubs_is_read_only_and_cursor_is_not_confused_with_destination() {
    let command = Command::ListHubs {
        after: Some(hub(SECOND_HUB_ID)),
    };
    assert!(!command.mutates());
    assert_eq!(command.request_id(), None);
    assert!(Request::new(hub(HUB_ID), command).is_ok());
}

#[test]
fn duplicate_unsorted_and_nonprogressing_hub_pages_are_rejected() {
    let request = request(Command::ListHubs { after: None });
    let one = summary(HUB_ID, true);
    let two = summary(SECOND_HUB_ID, false);
    let invalid = [
        Reply::Hubs {
            hubs: vec![two.clone(), one.clone()],
            next: None,
        },
        Reply::Hubs {
            hubs: vec![one.clone(), one.clone()],
            next: None,
        },
        Reply::Hubs {
            hubs: vec![
                one.clone(),
                HubSummary {
                    reachable: false,
                    label: None,
                    ..one.clone()
                },
            ],
            next: None,
        },
        Reply::Hubs {
            hubs: vec![one.clone(), two.clone()],
            next: Some(one.hub_id),
        },
        Reply::Hubs {
            hubs: vec![],
            next: Some(one.hub_id),
        },
    ];
    for reply in invalid {
        assert_eq!(
            Response::new(&request, hub(HUB_ID), reply).validate_for(&request),
            Err(PublicError::InvalidPayload)
        );
    }
    for next in [None, Some(two.hub_id)] {
        let reply = Reply::Hubs {
            hubs: vec![one.clone(), two.clone()],
            next,
        };
        assert_eq!(
            Response::new(&request, hub(HUB_ID), reply).validate_for(&request),
            Ok(())
        );
    }
}

#[test]
fn hub_page_size_and_after_cursor_are_enforced_at_exact_boundaries() {
    let request = request(Command::ListHubs { after: None });
    for count in [MAX_CATALOG_PAGE_HUBS, MAX_CATALOG_PAGE_HUBS + PAGE_STEP] {
        let hubs = (PAGE_STEP..=count)
            .map(|index| summary(index as u128, true))
            .collect();
        let response = Response::new(&request, hub(HUB_ID), Reply::Hubs { hubs, next: None });
        assert_eq!(
            response.validate_for(&request).is_ok(),
            count == MAX_CATALOG_PAGE_HUBS
        );
    }
    let request = crate::commands::request(Command::ListHubs {
        after: Some(hub(SECOND_HUB_ID)),
    });
    for candidate in [HUB_ID, SECOND_HUB_ID, SESSION_ID] {
        let reply = Reply::Hubs {
            hubs: vec![summary(candidate, false)],
            next: None,
        };
        assert_eq!(
            Response::new(&request, hub(HUB_ID), reply)
                .validate_for(&request)
                .is_ok(),
            candidate > SECOND_HUB_ID
        );
    }
    assert!(
        Response::new(
            &request,
            hub(HUB_ID),
            Reply::Hubs {
                hubs: vec![],
                next: None
            }
        )
        .validate_for(&request)
        .is_ok()
    );
}
