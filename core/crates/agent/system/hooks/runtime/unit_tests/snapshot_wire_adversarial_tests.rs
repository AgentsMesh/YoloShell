use super::ReducerSnapshot;
use super::constants::{snapshot_fields as fields, snapshot_indexes as indexes};
use super::snapshot_canonical_adversarial_tests::rich_claude_value;
use serde_json::{Map, Value, from_value};

#[test]
fn every_state_dto_rejects_unknown_fields() {
    let (_, original) = rich_claude_value();
    let paths: &[&[&str]] = &[
        &[fields::LIFECYCLE, fields::STOP_CANDIDATE],
        &[fields::LIFECYCLE, fields::CHILDREN, indexes::FIRST],
        &[
            fields::LIFECYCLE,
            fields::CHILDREN,
            indexes::FIRST,
            fields::LIFECYCLE,
        ],
        &[fields::LIFECYCLE, fields::COMPACTION],
        &[fields::LIFECYCLE, fields::CHILD_COMPACTIONS, indexes::FIRST],
        &[
            fields::LIFECYCLE,
            fields::CHILD_COMPACTIONS,
            indexes::FIRST,
            fields::PHASE,
        ],
        &[
            fields::HISTORY,
            fields::ENTRIES,
            indexes::NOTIFICATION,
            fields::EVENT,
            fields::NOTIFICATION,
        ],
        &[
            fields::HISTORY,
            fields::ENTRIES,
            indexes::SECOND,
            fields::EVENT,
        ],
    ];
    for path in paths {
        let mut value = original.clone();
        object_at(&mut value, path).insert(fields::UNKNOWN.to_owned(), Value::Bool(true));
        assert!(from_value::<ReducerSnapshot>(value).is_err(), "{path:?}");
    }
}

fn object_at<'a>(value: &'a mut Value, path: &[&str]) -> &'a mut Map<String, Value> {
    let mut current = value;
    for segment in path {
        current = if let Ok(index) = segment.parse::<usize>() {
            &mut current.as_array_mut().unwrap()[index]
        } else {
            &mut current[*segment]
        };
    }
    current.as_object_mut().unwrap()
}
