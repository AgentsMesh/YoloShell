use crate::instance_descriptor;
use crate::lcov::ParsedReport;
use crate::lcov_inventory::{add_source, marker_totals, presence};
use std::collections::BTreeMap;
use yolo_coverage_protocol::instance_inventory_digest;

#[derive(Debug, Default)]
pub struct InstanceInventories(BTreeMap<(String, String), InstanceInventory>);

#[derive(Debug, Default)]
struct InstanceInventory {
    header: Option<InventoryHeader>,
    instances: BTreeMap<String, u64>,
}

#[derive(Clone, Copy, Debug)]
struct InventoryHeader {
    count: u64,
    members: u64,
}

impl InstanceInventories {
    pub fn record_header(
        &mut self,
        source: &str,
        inventory: &str,
        members: u64,
        count: u64,
    ) -> Result<(), String> {
        presence(count, "instance inventory")?;
        let record = self.record(source, inventory);
        if record
            .header
            .replace(InventoryHeader { count, members })
            .is_some()
        {
            return Err("duplicate instance inventory header".to_owned());
        }
        Ok(())
    }

    pub fn record_member(
        &mut self,
        source: &str,
        inventory: &str,
        instance: &str,
        count: u64,
    ) -> Result<(), String> {
        presence(count, "instance inventory member")?;
        if self
            .record(source, inventory)
            .instances
            .insert(instance.to_owned(), count)
            .is_some()
        {
            return Err("duplicate instance inventory member".to_owned());
        }
        Ok(())
    }

    fn record(&mut self, source: &str, inventory: &str) -> &mut InstanceInventory {
        self.0
            .entry((source.to_owned(), inventory.to_owned()))
            .or_default()
    }
}

pub fn validate(report: &ParsedReport) -> Result<(), String> {
    let actual = instance_descriptor::collect(report)?;
    let mut header_totals = BTreeMap::<String, u64>::new();
    let mut member_totals = BTreeMap::<(String, String), u64>::new();
    for ((source, digest), inventory) in &report.instance_inventories.0 {
        if !report.production.contains(source) {
            return Err("instance inventory appears outside production sources".to_owned());
        }
        let header = inventory
            .header
            .ok_or_else(|| "instance inventory has no header".to_owned())?;
        let length = u64::try_from(inventory.instances.len())
            .map_err(|_| "instance inventory length overflow")?;
        if length != header.members {
            return Err("instance inventory length differs from its header".to_owned());
        }
        let members = inventory
            .instances
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if instance_inventory_digest(&members)? != *digest {
            return Err("instance inventory digest mismatch".to_owned());
        }
        add_source(
            &mut header_totals,
            source,
            header.count,
            "instance inventory multiplicity",
        )?;
        for (instance, count) in &inventory.instances {
            if *count != header.count {
                return Err("instance inventory member multiplicity mismatch".to_owned());
            }
            add_member(&mut member_totals, source, instance, *count)?;
        }
    }
    marker_totals(report, &header_totals, "instance inventory")?;
    instance_descriptor::compare(&actual, &member_totals)
}

fn add_member(
    totals: &mut BTreeMap<(String, String), u64>,
    source: &str,
    instance: &str,
    count: u64,
) -> Result<(), String> {
    let total = totals
        .entry((source.to_owned(), instance.to_owned()))
        .or_default();
    *total = total
        .checked_add(count)
        .ok_or_else(|| "instance member multiplicity overflow".to_owned())?;
    Ok(())
}
