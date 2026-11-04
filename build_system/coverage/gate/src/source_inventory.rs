use crate::lcov::ParsedReport;
use crate::lcov_inventory::{add_source, marker_totals, presence};
use std::collections::{BTreeMap, BTreeSet};
use yolo_coverage_protocol::source_inventory_digest;

#[derive(Debug, Default)]
pub struct SourceInventories(BTreeMap<String, SourceInventory>);

#[derive(Debug)]
struct SourceInventory {
    members: u64,
    sources: BTreeMap<String, u64>,
}

impl SourceInventories {
    pub fn record(
        &mut self,
        inventory: &str,
        members: u64,
        source: &str,
        count: u64,
    ) -> Result<(), String> {
        presence(count, "source inventory")?;
        let record = self
            .0
            .entry(inventory.to_owned())
            .or_insert_with(|| SourceInventory {
                members,
                sources: BTreeMap::new(),
            });
        if record.members != members {
            return Err("source inventory member counts disagree".to_owned());
        }
        if record.sources.insert(source.to_owned(), count).is_some() {
            return Err("duplicate source inventory token".to_owned());
        }
        Ok(())
    }
}

pub fn validate(report: &ParsedReport) -> Result<(), String> {
    let marked = report.markers.keys().cloned().collect::<BTreeSet<_>>();
    if report.production != marked {
        return Err(format!(
            "coverage marker sources differ from production sources: {marked:?} != {:?}",
            report.production
        ));
    }
    let mut totals = BTreeMap::<String, u64>::new();
    for (digest, inventory) in &report.source_inventories.0 {
        let length = u64::try_from(inventory.sources.len())
            .map_err(|_| "source inventory length overflow")?;
        if length != inventory.members {
            return Err(format!(
                "source inventory length differs from its header: {length}/{} for {digest}",
                inventory.members
            ));
        }
        let members = inventory
            .sources
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if source_inventory_digest(&members)? != *digest {
            return Err("source inventory digest mismatch".to_owned());
        }
        let expected = inventory
            .sources
            .values()
            .next()
            .copied()
            .ok_or_else(|| "source inventory has no member locations".to_owned())?;
        for (source, count) in &inventory.sources {
            if !report.production.contains(source) || *count != expected {
                return Err("source inventory membership is inconsistent".to_owned());
            }
            add_source(&mut totals, source, *count, "source inventory multiplicity")?;
        }
    }
    marker_totals(report, &totals, "source inventory")
}
