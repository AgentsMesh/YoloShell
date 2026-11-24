use std::collections::BTreeMap;
use yolo_architecture_foundation::{SourceFile, model::Violation};

const FLAT_FILE_LIMIT: usize = 40;
const POLICY_MESSAGE: &str = "directory exceeds the flat-file budget; split it by feature";

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let mut counts = BTreeMap::<&str, usize>::new();
    for file in files {
        let Some(directory) = file
            .relative
            .rsplit_once('/')
            .map(|(directory, _)| directory)
        else {
            continue;
        };
        if !handwritten(&file.relative) {
            continue;
        }
        *counts.entry(directory).or_default() += 1;
    }
    for (directory, count) in counts {
        if count <= FLAT_FILE_LIMIT {
            continue;
        }
        let baseline = baseline(directory);
        if baseline.is_some_and(|baseline| count <= baseline) {
            continue;
        }
        let detail = baseline.map_or_else(
            || format!("{count} files, limit {FLAT_FILE_LIMIT}"),
            |baseline| format!("{count} files, baseline {baseline}, limit {FLAT_FILE_LIMIT}"),
        );
        violations.push(Violation::file(
            directory,
            format!("{POLICY_MESSAGE}: {detail}"),
        ));
    }
}

fn handwritten(path: &str) -> bool {
    path.ends_with(".rs") || path.ends_with(".bzl")
}

fn baseline(directory: &str) -> Option<usize> {
    BASELINE
        .iter()
        .find(|(entry, _)| *entry == directory)
        .map(|(_, count)| *count)
}

const BASELINE: &[(&str, usize)] = &[];
