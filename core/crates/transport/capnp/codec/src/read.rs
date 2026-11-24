use crate::budget::Budget;
use crate::pending::Pending;
use crate::{Value, ValueError, constants::*, error};
use yolo_capnp_schema::value_capnp::value;

pub(crate) fn value(
    reader: value::Reader<'_>,
    depth: usize,
    budget: &mut Budget,
    cleanup: fn(&mut Value),
) -> Result<Value, ValueError> {
    if depth > MAX_VALUE_DEPTH {
        return Err(ValueError::Depth);
    }
    budget.node()?;
    Ok(match reader.which().map_err(error)? {
        value::Null(()) => Value::Null,
        value::Boolean(value) => Value::Boolean(value),
        value::Signed(value) => Value::Signed(value),
        value::Unsigned(value) => Value::Unsigned(value),
        value::Float(value) => Value::Float(value),
        value::Text(value) => {
            let text = value
                .map_err(error)?
                .to_str()
                .map_err(|_| ValueError::Utf8)?;
            budget.blob(text.len(), true)?;
            Value::Text(text.into())
        }
        value::Data(value) => {
            let data = value.map_err(error)?;
            budget.blob(data.len(), false)?;
            Value::Data(data.into())
        }
        value::List(values) => {
            let values = values.map_err(error)?;
            budget.list(values.len() as usize, false)?;
            let mut pending = Pending::new(
                Value::List(Vec::with_capacity(values.len() as usize)),
                cleanup,
            );
            if let Value::List(items) = &mut pending.value {
                for item in values {
                    items.push(value(item, depth + CHILD_DEPTH, budget, cleanup)?);
                }
            }
            pending.take()
        }
        value::Map(values) => {
            let values = values.map_err(error)?;
            budget.list(values.len() as usize, true)?;
            let mut pending = Pending::new(
                Value::Map(Vec::with_capacity(values.len() as usize)),
                cleanup,
            );
            if let Value::Map(entries) = &mut pending.value {
                for item in values {
                    let key = Pending::new(
                        value(
                            item.get_key().map_err(error)?,
                            depth + CHILD_DEPTH,
                            budget,
                            cleanup,
                        )?,
                        cleanup,
                    );
                    let child = value(
                        item.get_value().map_err(error)?,
                        depth + CHILD_DEPTH,
                        budget,
                        cleanup,
                    )?;
                    entries.push((key.take(), child));
                }
            }
            pending.take()
        }
    })
}
