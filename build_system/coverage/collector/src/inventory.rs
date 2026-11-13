use crate::model::Instance;
use std::collections::{BTreeMap, BTreeSet};
use yolo_coverage_protocol::{
    MAXIMUM_INSTANCE_VECTOR_LENGTH, MetricKind, MetricShape, instance_digest,
    instance_inventory_digest, source_inventory_digest,
};

pub struct SetInventory {
    pub digest: String,
    pub members: u64,
}

pub struct InstanceInventory {
    pub members: Vec<String>,
    pub set: SetInventory,
}

pub fn sources(sources: &BTreeSet<String>) -> Result<SetInventory, String> {
    let members = bounded_len(sources.len(), "source inventory")?;
    let fields = sources.iter().map(String::as_str).collect::<Vec<_>>();
    Ok(SetInventory {
        digest: source_inventory_digest(&fields)?,
        members,
    })
}

pub fn instances(instances: &[Instance]) -> Result<InstanceInventory, String> {
    let mut members = BTreeMap::new();
    for instance in instances {
        let digest = instance_digest(
            instance.line,
            instance.column,
            &instance.identity,
            &instance.signature,
            &shapes(instance)?,
        );
        if members.insert(digest, ()).is_some() {
            return Err("instance inventory contains a duplicate digest".to_owned());
        }
    }
    let members = members.into_keys().collect::<Vec<_>>();
    let count = bounded_len(members.len(), "instance inventory")?;
    let fields = members.iter().map(String::as_str).collect::<Vec<_>>();
    Ok(InstanceInventory {
        set: SetInventory {
            digest: instance_inventory_digest(&fields)?,
            members: count,
        },
        members,
    })
}

pub fn shapes(instance: &Instance) -> Result<[MetricShape; 4], String> {
    Ok([
        shape(instance, MetricKind::Lines)?,
        shape(instance, MetricKind::Functions)?,
        shape(instance, MetricKind::Regions)?,
        shape(instance, MetricKind::Branches)?,
    ])
}

fn shape(instance: &Instance, kind: MetricKind) -> Result<MetricShape, String> {
    let vector_len = instance
        .vectors
        .get(&kind)
        .ok_or_else(|| format!("missing {} inventory vector", kind.label()))?
        .len();
    let vector_len = bounded_len(vector_len, kind.label())?;
    let found = if kind == MetricKind::Branches {
        instance.branch_found
    } else {
        vector_len
    };
    if found > vector_len {
        return Err(format!("{} denominator exceeds its vector", kind.label()));
    }
    Ok(MetricShape { found, vector_len })
}

fn bounded_len(value: usize, label: &str) -> Result<u64, String> {
    let value = u64::try_from(value).map_err(|_| format!("{label} length overflow"))?;
    (value <= MAXIMUM_INSTANCE_VECTOR_LENGTH)
        .then_some(value)
        .ok_or_else(|| format!("{label} exceeds the protocol limit"))
}
