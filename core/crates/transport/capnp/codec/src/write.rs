use crate::budget::Budget;
use crate::{Value, ValueError, constants::*};
use yolo_capnp_schema::value_capnp::value;

pub(crate) fn value(
    mut builder: value::Builder<'_>,
    source: &Value,
    depth: usize,
    budget: &mut Budget,
) -> Result<(), ValueError> {
    if depth > MAX_VALUE_DEPTH {
        return Err(ValueError::Depth);
    }
    budget.node()?;
    match source {
        Value::Null => builder.set_null(()),
        Value::Boolean(value) => builder.set_boolean(*value),
        Value::Signed(value) => builder.set_signed(*value),
        Value::Unsigned(value) => builder.set_unsigned(*value),
        Value::Float(value) => builder.set_float(*value),
        Value::Text(value) => {
            budget.blob(value.len(), true)?;
            builder.set_text(value.as_str());
        }
        Value::Data(value) => {
            budget.blob(value.len(), false)?;
            builder.set_data(value);
        }
        Value::List(values) => {
            let count = budget.list(values.len(), false)?;
            let mut list = builder.init_list(count);
            for (index, source) in values.iter().enumerate() {
                value(
                    list.reborrow().get(index as u32),
                    source,
                    depth + CHILD_DEPTH,
                    budget,
                )?;
            }
        }
        Value::Map(values) => {
            let count = budget.list(values.len(), true)?;
            let mut map = builder.init_map(count);
            for (index, (key, source)) in values.iter().enumerate() {
                let mut item = map.reborrow().get(index as u32);
                value(item.reborrow().init_key(), key, depth + CHILD_DEPTH, budget)?;
                value(item.init_value(), source, depth + CHILD_DEPTH, budget)?;
            }
        }
    }
    Ok(())
}
