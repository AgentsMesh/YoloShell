use crate::lcov::{Header, InstanceRecords, LogicalKey, ParsedReport, Record};
use crate::source::production_key;
use yolo_coverage_protocol::{FunctionToken, parse_token};

pub(super) fn finish(record: Record<'_>, parsed: &mut ParsedReport) -> Result<(), String> {
    let source = record.source.strip_prefix("./").unwrap_or(record.source);
    let production = production_key(source)?;
    let entries = crate::lcov_record::synthetic(&record.lines)?;
    let Some(source) = production else {
        if entries.is_empty() {
            return Ok(());
        }
        return Err("synthetic function token appears outside production Rust".to_owned());
    };
    if entries.is_empty() && crate::lcov_record::empty_placeholder(&record.lines) {
        return Ok(());
    }
    if !parsed.production.insert(source.clone()) {
        return Err(format!("duplicate production LCOV source {source}"));
    }
    for entry in entries {
        dispatch(parsed, &source, entry)?;
    }
    Ok(())
}

fn dispatch(
    parsed: &mut ParsedReport,
    source: &str,
    entry: crate::lcov_record::FunctionEntry,
) -> Result<(), String> {
    let token = parse_token(&entry.name)?
        .ok_or_else(|| "internal synthetic token classification failure".to_owned())?;
    crate::lcov_record::validate_location(&entry, &token)?;
    match token {
        FunctionToken::Marker => marker(parsed, source, entry.count),
        FunctionToken::SourceInventory { inventory, members } => {
            parsed
                .source_inventories
                .record(inventory, members, source, entry.count)
        }
        FunctionToken::InstanceInventory { inventory, members } => parsed
            .instance_inventories
            .record_header(source, inventory, members, entry.count),
        FunctionToken::InstanceMember {
            inventory,
            instance,
        } => parsed
            .instance_inventories
            .record_member(source, inventory, instance, entry.count),
        FunctionToken::Header {
            kind,
            line,
            column,
            identity,
            signature,
            vector_len,
            found,
        } => {
            crate::lcov_inventory::presence(entry.count, "synthetic metric header")?;
            let records = instance_records(parsed, source, line, column, identity, signature)?;
            if records
                .headers
                .insert(
                    kind,
                    Header {
                        count: entry.count,
                        found,
                        vector_len,
                    },
                )
                .is_some()
            {
                return Err("duplicate synthetic metric header".to_owned());
            }
            records.ordinals.entry(kind).or_default();
            Ok(())
        }
        FunctionToken::Ordinal {
            kind,
            line,
            column,
            identity,
            signature,
            ordinal,
        } => {
            let records = instance_records(parsed, source, line, column, identity, signature)?;
            if records
                .ordinals
                .entry(kind)
                .or_default()
                .insert(ordinal, entry.count)
                .is_some()
            {
                return Err("duplicate synthetic metric ordinal".to_owned());
            }
            Ok(())
        }
    }
}

fn marker(parsed: &mut ParsedReport, source: &str, count: u64) -> Result<(), String> {
    crate::lcov_inventory::presence(count, "coverage marker")?;
    if parsed.markers.insert(source.to_owned(), count).is_some() {
        return Err(format!("invalid or duplicate coverage marker for {source}"));
    }
    Ok(())
}

fn instance_records<'a>(
    parsed: &'a mut ParsedReport,
    source: &str,
    line: u64,
    column: u64,
    identity: &str,
    signature: &str,
) -> Result<&'a mut InstanceRecords, String> {
    let key = LogicalKey {
        column,
        identity: identity.to_owned(),
        line,
        source: source.to_owned(),
    };
    let records = parsed.instances.entry(key).or_default();
    if records.signature.is_empty() {
        records.signature = signature.to_owned();
    } else if records.signature != signature {
        return Err("one coverage instance has incompatible static maps".to_owned());
    }
    Ok(records)
}
