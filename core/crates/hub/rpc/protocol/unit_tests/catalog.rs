use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use yolo_hub_rpc_protocol::{Command, MAX_CATALOG_PAGE_TERMINALS, Reply, Response};
use yolo_hub_terminal_protocol::{PublicError, TerminalSummary};

#[test]
fn retired_collection_fields_and_reply_tags_never_decode_as_an_empty_catalog() {
    let current = serde_json::to_value(Reply::Terminals {
        terminals: Vec::new(),
        next: None,
    })
    .unwrap();
    let mut old_tag = current.clone();
    old_tag[REPLY_RESULT_FIELD] = REMOVED_TERMINAL_COLLECTION.into();
    let mut old_field = current.clone();
    old_field.as_object_mut().unwrap().remove(TERMINALS_FIELD);
    old_field[REMOVED_TERMINAL_COLLECTION] = serde_json::json!([]);
    let mut absent = current;
    absent.as_object_mut().unwrap().remove(TERMINALS_FIELD);
    for wire in [old_tag, old_field, absent] {
        assert!(serde_json::from_value::<Reply>(wire).is_err());
    }
}

#[test]
fn mixed_origin_pages_are_sorted_by_stable_address_and_cursor_is_last_item() {
    let request = request(Command::List {
        include_archived: false,
        after: None,
    });
    let one = summary(HUB_ID, SESSION_ID);
    let two = summary(SECOND_HUB_ID, SESSION_ID);
    let valid = Reply::Terminals {
        terminals: vec![one.clone(), two.clone()],
        next: Some(two.fields().address),
    };
    assert_eq!(
        Response::new(&request, hub(HUB_ID), valid).validate_for(&request),
        Ok(())
    );
    let invalid = [
        Reply::Terminals {
            terminals: vec![two.clone(), one.clone()],
            next: None,
        },
        Reply::Terminals {
            terminals: vec![one.clone(), one.clone()],
            next: None,
        },
        Reply::Terminals {
            terminals: vec![one.clone(), two],
            next: Some(one.fields().address),
        },
        Reply::Terminals {
            terminals: Vec::new(),
            next: Some(one.fields().address),
        },
    ];
    for reply in invalid {
        assert_eq!(
            Response::new(&request, hub(HUB_ID), reply).validate_for(&request),
            Err(PublicError::InvalidPayload)
        );
    }
    assert_eq!(
        Response::new(
            &request,
            hub(HUB_ID),
            Reply::Terminals {
                terminals: Vec::new(),
                next: None
            }
        )
        .validate_for(&request),
        Ok(())
    );
}

#[test]
fn pages_cannot_regress_cursor_or_sneak_archived_entries_into_active_view() {
    let after = address(HUB_ID, SESSION_ID);
    for include_archived in [false, true] {
        let request = request(Command::List {
            include_archived,
            after: Some(after),
        });
        let mut archived_fields = summary(HUB_ID, SECOND_SESSION_ID).fields().clone();
        archived_fields.archived = true;
        let archived = TerminalSummary::new(archived_fields).unwrap();
        let response = Response::new(
            &request,
            hub(HUB_ID),
            Reply::Terminals {
                terminals: vec![archived],
                next: None,
            },
        );
        assert_eq!(response.validate_for(&request).is_ok(), include_archived);
        for session in [summary(HUB_ID, HUB_ID), summary(HUB_ID, SESSION_ID)] {
            let response = Response::new(
                &request,
                hub(HUB_ID),
                Reply::Terminals {
                    terminals: vec![session],
                    next: None,
                },
            );
            assert_eq!(
                response.validate_for(&request),
                Err(PublicError::InvalidPayload)
            );
        }
        let response = Response::new(
            &request,
            hub(HUB_ID),
            Reply::Terminals {
                terminals: vec![summary(SECOND_HUB_ID, SESSION_ID)],
                next: None,
            },
        );
        assert_eq!(response.validate_for(&request), Ok(()));
    }
}

#[test]
fn catalog_page_limit_is_enforced_at_the_exact_boundary() {
    let request = request(Command::List {
        include_archived: false,
        after: None,
    });
    for count in [
        MAX_CATALOG_PAGE_TERMINALS,
        MAX_CATALOG_PAGE_TERMINALS + PAGE_STEP,
    ] {
        let terminals = (PAGE_STEP..=count)
            .map(|index| summary(HUB_ID, index as u128))
            .collect();
        let response = Response::new(
            &request,
            hub(HUB_ID),
            Reply::Terminals {
                terminals,
                next: None,
            },
        );
        assert_eq!(
            response.validate_for(&request).is_ok(),
            count == MAX_CATALOG_PAGE_TERMINALS
        );
    }
}
