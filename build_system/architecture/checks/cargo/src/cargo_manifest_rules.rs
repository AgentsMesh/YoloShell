use crate::cargo_manifest_syntax::{Document, Entry, Value};
use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    use crate::policy_constants::*;
    let document = crate::cargo_manifest_syntax::parse(&file.text);
    for line in &document.unsupported_lines {
        violations.push(Violation::line(
            &file.relative,
            *line,
            CARGO_METADATA_SHAPE_MESSAGE,
        ));
    }
    inspect_tables(file, &document, violations);
    inspect_dotted_root_entries(file, &document, violations);
    inspect_false_entries(
        file,
        &document,
        CARGO_PACKAGE_TABLE,
        CARGO_REQUIRED_PACKAGE_FALSE_KEYS,
        violations,
    );
    inspect_false_entries(
        file,
        &document,
        CARGO_LIBRARY_TABLE,
        CARGO_REQUIRED_LIBRARY_FALSE_KEYS,
        violations,
    );
    for (key, value) in CARGO_REQUIRED_LIBRARY_STRINGS {
        inspect_entry(
            file,
            &document.entries,
            CARGO_LIBRARY_TABLE,
            key,
            &Value::Text((*value).to_owned()),
            violations,
        );
    }
}

fn inspect_tables(file: &SourceFile, document: &Document, violations: &mut Vec<Violation>) {
    use crate::policy_constants::*;
    for table in &document.tables {
        if forbidden_table(&table.name) {
            violations.push(Violation::line(
                &file.relative,
                table.line,
                CARGO_PRODUCTION_TARGET_MESSAGE,
            ));
        } else if table.array
            && matches!(
                table.name.as_str(),
                CARGO_PACKAGE_TABLE | CARGO_LIBRARY_TABLE
            )
        {
            report_shape(file, Some(table.line), violations);
        }
    }
    for required in [CARGO_PACKAGE_TABLE, CARGO_LIBRARY_TABLE] {
        let matching = document
            .tables
            .iter()
            .filter(|table| table.name == required && !table.array)
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            report_shape(file, matching.first().map(|table| table.line), violations);
        }
    }
}

fn inspect_dotted_root_entries(
    file: &SourceFile,
    document: &Document,
    violations: &mut Vec<Violation>,
) {
    use crate::policy_constants::*;
    for entry in document
        .entries
        .iter()
        .filter(|entry| entry.table.is_empty())
    {
        let root = entry.key.split('.').next().unwrap_or_default();
        if forbidden_table(root) {
            violations.push(Violation::line(
                &file.relative,
                entry.line,
                CARGO_PRODUCTION_TARGET_MESSAGE,
            ));
        } else if matches!(root, CARGO_PACKAGE_TABLE | CARGO_LIBRARY_TABLE) {
            report_shape(file, Some(entry.line), violations);
        }
    }
}

fn inspect_false_entries(
    file: &SourceFile,
    document: &Document,
    table: &str,
    keys: &[&str],
    violations: &mut Vec<Violation>,
) {
    for key in keys {
        inspect_entry(
            file,
            &document.entries,
            table,
            key,
            &Value::False,
            violations,
        );
    }
}

fn inspect_entry(
    file: &SourceFile,
    entries: &[Entry],
    table: &str,
    key: &str,
    expected: &Value,
    violations: &mut Vec<Violation>,
) {
    let matching = entries
        .iter()
        .filter(|entry| entry.table == table && entry.key == key)
        .collect::<Vec<_>>();
    if matching.len() == 1 && matching[0].value == *expected {
        return;
    }
    report_shape(file, matching.first().map(|entry| entry.line), violations);
}

fn forbidden_table(table: &str) -> bool {
    use crate::policy_constants::*;
    CARGO_FORBIDDEN_TABLES.contains(&table)
        || CARGO_FORBIDDEN_TABLE_PREFIXES
            .iter()
            .any(|prefix| table.starts_with(prefix))
}

fn report_shape(file: &SourceFile, line: Option<usize>, violations: &mut Vec<Violation>) {
    let message = crate::policy_constants::CARGO_METADATA_SHAPE_MESSAGE;
    match line {
        Some(line) => violations.push(Violation::line(&file.relative, line, message)),
        None => violations.push(Violation::file(&file.relative, message)),
    }
}
