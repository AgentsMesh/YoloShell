use crate::inventory::{SetInventory, instances, shapes};
use crate::model::Instance;
use yolo_coverage_protocol::{
    MARKER_TOKEN, MINIMUM_SOURCE_POSITION, MetricKind, header_token, instance_inventory_token,
    instance_member_token, ordinal_token, source_inventory_token,
};

const PRESENCE_COUNT: u64 = 1;

pub struct SyntheticFunction {
    pub count: u64,
    pub line: u64,
    pub name: String,
}

pub fn functions(
    source_inventory: &SetInventory,
    source_instances: &[Instance],
) -> Result<Vec<SyntheticFunction>, String> {
    let inventory = instances(source_instances)?;
    let mut output = vec![presence(MARKER_TOKEN.to_owned())];
    output.push(presence(source_inventory_token(
        &source_inventory.digest,
        source_inventory.members,
    )));
    output.push(presence(instance_inventory_token(
        &inventory.set.digest,
        inventory.set.members,
    )));
    for member in &inventory.members {
        output.push(presence(instance_member_token(
            &inventory.set.digest,
            member,
        )));
    }
    for instance in source_instances {
        append_instance(&mut output, instance)?;
    }
    Ok(output)
}

fn append_instance(output: &mut Vec<SyntheticFunction>, instance: &Instance) -> Result<(), String> {
    let shapes = shapes(instance)?;
    for (index, kind) in MetricKind::ALL.into_iter().enumerate() {
        let counts = instance
            .vectors
            .get(&kind)
            .ok_or_else(|| format!("missing {} vector for {}", kind.label(), instance.name))?;
        let shape = shapes[index];
        output.push(SyntheticFunction {
            count: PRESENCE_COUNT,
            line: instance.line,
            name: header_token(
                kind,
                instance.line,
                instance.column,
                &instance.identity,
                &instance.signature,
                shape.vector_len,
                shape.found,
            ),
        });
        for (ordinal, count) in counts.iter().copied().enumerate() {
            output.push(SyntheticFunction {
                count: presence_count(count),
                line: instance.line,
                name: ordinal_token(
                    kind,
                    instance.line,
                    instance.column,
                    &instance.identity,
                    &instance.signature,
                    u64::try_from(ordinal).map_err(|_| "metric ordinal overflow")?,
                ),
            });
        }
    }
    Ok(())
}

fn presence(name: String) -> SyntheticFunction {
    SyntheticFunction {
        count: PRESENCE_COUNT,
        line: MINIMUM_SOURCE_POSITION,
        name,
    }
}

fn presence_count(count: u64) -> u64 {
    if count > u64::default() {
        PRESENCE_COUNT
    } else {
        u64::default()
    }
}
