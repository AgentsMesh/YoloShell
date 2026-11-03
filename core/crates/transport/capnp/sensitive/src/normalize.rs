use std::cmp::Ordering;
use yolo_capnp_codec::{Value, ValueError};

// Sort borrowed keys directly: serde conversion would create unsanitized copies.
pub(crate) fn unique(value: &mut Value) -> Result<(), ValueError> {
    match value {
        Value::List(values) => {
            for value in values {
                unique(value)?;
            }
        }
        Value::Map(values) => {
            for (key, value) in values.iter_mut() {
                unique(key)?;
                unique(value)?;
            }
            values.sort_unstable_by(|left, right| compare(&left.0, &right.0));
            if values
                .windows(2)
                .any(|pair| compare(&pair[0].0, &pair[1].0).is_eq())
            {
                return Err(ValueError::DuplicateKey);
            }
        }
        _ => {}
    }
    Ok(())
}

fn compare(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Boolean(left), Value::Boolean(right)) => left.cmp(right),
        (Value::Signed(left), Value::Signed(right)) => left.cmp(right),
        (Value::Unsigned(left), Value::Unsigned(right)) => left.cmp(right),
        (Value::Float(left), Value::Float(right)) => left
            .partial_cmp(right)
            .unwrap_or_else(|| left.is_nan().cmp(&right.is_nan())),
        (Value::Text(left), Value::Text(right)) => left.cmp(right),
        (Value::Data(left), Value::Data(right)) => left.cmp(right),
        (Value::List(left), Value::List(right)) => left
            .iter()
            .zip(right)
            .map(|(left, right)| compare(left, right))
            .find(|order| !order.is_eq())
            .unwrap_or_else(|| left.len().cmp(&right.len())),
        (Value::Map(left), Value::Map(right)) => left
            .iter()
            .zip(right)
            .map(|(left, right)| {
                compare(&left.0, &right.0).then_with(|| compare(&left.1, &right.1))
            })
            .find(|order| !order.is_eq())
            .unwrap_or_else(|| left.len().cmp(&right.len())),
        _ => kind(left).cmp(&kind(right)),
    }
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum Kind {
    Null,
    Boolean,
    Signed,
    Unsigned,
    Float,
    Text,
    Data,
    List,
    Map,
}
fn kind(value: &Value) -> Kind {
    match value {
        Value::Null => Kind::Null,
        Value::Boolean(_) => Kind::Boolean,
        Value::Signed(_) => Kind::Signed,
        Value::Unsigned(_) => Kind::Unsigned,
        Value::Float(_) => Kind::Float,
        Value::Text(_) => Kind::Text,
        Value::Data(_) => Kind::Data,
        Value::List(_) => Kind::List,
        Value::Map(_) => Kind::Map,
    }
}
