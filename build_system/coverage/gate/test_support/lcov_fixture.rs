use std::collections::{BTreeMap, BTreeSet};
use yolo_coverage_protocol::{
    FunctionToken, MARKER_TOKEN, MINIMUM_SOURCE_POSITION, MetricKind, MetricShape, instance_digest,
    instance_inventory_digest, instance_inventory_token, instance_member_token, parse_token,
    source_inventory_digest, source_inventory_token,
};

pub type Entry = (u64, String, u64);
type InstanceKey = (u64, u64, String, String);
const PRESENCE_COUNT: u64 = 1;

#[derive(Clone, Copy)]
pub enum Decorations {
    Complete,
    MarkerAndSourceInventory,
    MarkerOnly,
    None,
}

pub fn report(
    source: &str,
    entries: &[Entry],
    inventory_entries: &[Entry],
    decorations: Decorations,
) -> String {
    report_with_sources(source, entries, inventory_entries, decorations, &[source])
}

pub fn report_with_sources(
    source: &str,
    entries: &[Entry],
    inventory_entries: &[Entry],
    decorations: Decorations,
    inventory_sources: &[&str],
) -> String {
    let mut all = Vec::new();
    if !matches!(decorations, Decorations::None) {
        all.push(presence(MARKER_TOKEN.to_owned()));
    }
    if matches!(
        decorations,
        Decorations::Complete | Decorations::MarkerAndSourceInventory
    ) {
        let inventories =
            inventories(inventory_sources, inventory_entries).expect("valid coverage fixture");
        all.push(inventories.source);
        if matches!(decorations, Decorations::Complete) {
            all.extend(inventories.instances);
        }
    }
    all.extend_from_slice(entries);
    let mut output = format!("SF:{source}\nDA:1,99\nLF:1\nLH:1\nBRDA:1,0,0,99\nBRF:1\nBRH:1\n");
    for (line, name, _) in &all {
        output.push_str(&format!("FN:{line},{name}\n"));
    }
    for (_, name, count) in &all {
        output.push_str(&format!("FNDA:{count},{name}\n"));
    }
    output.push_str(&format!(
        "FNF:{}\nFNH:{}\nend_of_record\n",
        all.len(),
        all.len()
    ));
    output
}

struct Inventories {
    instances: Vec<Entry>,
    source: Entry,
}

fn inventories(sources: &[&str], entries: &[Entry]) -> Result<Inventories, String> {
    let mut descriptors = BTreeMap::<InstanceKey, BTreeMap<MetricKind, MetricShape>>::new();
    for (_, name, _) in entries {
        if let Some(FunctionToken::Header {
            kind,
            line,
            column,
            identity,
            signature,
            vector_len,
            found,
        }) = parse_token(name)?
        {
            descriptors
                .entry((line, column, identity.to_owned(), signature.to_owned()))
                .or_default()
                .insert(kind, MetricShape { found, vector_len });
        }
    }
    let mut members = BTreeSet::new();
    for ((line, column, identity, signature), shapes) in descriptors {
        let shape = |kind| {
            shapes
                .get(&kind)
                .copied()
                .ok_or_else(|| format!("fixture has no {} shape", kind.label()))
        };
        members.insert(instance_digest(
            line,
            column,
            &identity,
            &signature,
            &[
                shape(MetricKind::Lines)?,
                shape(MetricKind::Functions)?,
                shape(MetricKind::Regions)?,
                shape(MetricKind::Branches)?,
            ],
        ));
    }
    let source_digest = source_inventory_digest(sources)?;
    let source_count = u64::try_from(sources.len()).map_err(|_| "fixture is too large")?;
    let member_fields = members.iter().map(String::as_str).collect::<Vec<_>>();
    let instance_digest = instance_inventory_digest(&member_fields)?;
    let member_count = u64::try_from(members.len()).map_err(|_| "fixture is too large")?;
    let mut instances = vec![presence(instance_inventory_token(
        &instance_digest,
        member_count,
    ))];
    instances.extend(
        members
            .into_iter()
            .map(|member| presence(instance_member_token(&instance_digest, &member))),
    );
    Ok(Inventories {
        instances,
        source: presence(source_inventory_token(&source_digest, source_count)),
    })
}

fn presence(name: String) -> Entry {
    (MINIMUM_SOURCE_POSITION, name, PRESENCE_COUNT)
}
