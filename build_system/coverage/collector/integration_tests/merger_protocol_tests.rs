use crate::constants::{
    BAZEL_COVERAGE_DIRECTORY_ARGUMENT, BAZEL_OUTPUT_FILE_ARGUMENT, BAZEL_SOURCE_MANIFEST_ARGUMENT,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use yolo_coverage_protocol::{
    MARKER_TOKEN, MetricKind, MetricShape, header_token, instance_digest,
    instance_inventory_digest, instance_inventory_token, instance_member_token, ordinal_token,
    source_inventory_digest, source_inventory_token,
};

const MERGER_ENV: &str = "YOLO_LCOV_MERGER";
const TEMP_DIRECTORY_ENV: &str = "TEST_TMPDIR";
const SOURCE: &str = "core/src/lib.rs";
const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const FIRST_REPORT: &str = "first.dat";
const SECOND_REPORT: &str = "second.dat";
const OUTPUT_REPORT: &str = "merged.dat";
const INPUT_DIRECTORY: &str = "inputs";
const SOURCE_MANIFEST: &str = "sources.txt";
const GROUP_LINE: u64 = 1;
const GROUP_COLUMN: u64 = 1;
const PRESENCE_COUNT: u64 = 1;
const MERGED_REPORT_COUNT: u64 = 2;
const FULL_VECTOR_LENGTH: u64 = 2;
const FUNCTION_VECTOR_LENGTH: u64 = 1;

#[test]
fn bazel_merger_preserves_markers_and_unions_instance_ordinals() {
    let temporary = PathBuf::from(std::env::var_os(TEMP_DIRECTORY_ENV).unwrap());
    let inputs = temporary.join(INPUT_DIRECTORY);
    std::fs::create_dir_all(&inputs).unwrap();
    std::fs::write(inputs.join(FIRST_REPORT), vector(&[1, 0])).unwrap();
    std::fs::write(inputs.join(SECOND_REPORT), vector(&[0, 1])).unwrap();
    let output = temporary.join(OUTPUT_REPORT);
    let manifest = temporary.join(SOURCE_MANIFEST);
    std::fs::write(&manifest, format!("{SOURCE}\n")).unwrap();
    let status = Command::new(std::env::var_os(MERGER_ENV).unwrap())
        .arg(argument(BAZEL_COVERAGE_DIRECTORY_ARGUMENT, &inputs))
        .arg(argument(BAZEL_OUTPUT_FILE_ARGUMENT, &output))
        .arg(argument(BAZEL_SOURCE_MANIFEST_ARGUMENT, &manifest))
        .status()
        .unwrap();
    assert!(status.success());
    let merged = std::fs::read_to_string(output).unwrap();
    for token in [MARKER_TOKEN.to_owned()]
        .into_iter()
        .chain(inventory_tokens().unwrap())
    {
        assert!(merged.contains(&format!("FNDA:{MERGED_REPORT_COUNT},{token}")));
    }
    for kind in MetricKind::ALL {
        let first_count = if kind == MetricKind::Functions {
            MERGED_REPORT_COUNT
        } else {
            PRESENCE_COUNT
        };
        assert!(merged.contains(&format!("FNDA:{first_count},{}", ordinal(kind, 0))));
        if kind != MetricKind::Functions {
            assert!(merged.contains(&format!("FNDA:1,{}", ordinal(kind, 1))));
        }
    }
}

fn vector(counts: &[u64]) -> String {
    let mut functions = vec![(MARKER_TOKEN.to_owned(), PRESENCE_COUNT)];
    functions.extend(
        inventory_tokens()
            .unwrap()
            .into_iter()
            .map(|token| (token, PRESENCE_COUNT)),
    );
    for kind in MetricKind::ALL {
        let metric_counts = match kind {
            MetricKind::Functions => &[PRESENCE_COUNT][..],
            _ => counts,
        };
        let length = u64::try_from(metric_counts.len()).unwrap();
        functions.push((
            header_token(
                kind,
                GROUP_LINE,
                GROUP_COLUMN,
                DIGEST,
                DIGEST,
                length,
                length,
            ),
            PRESENCE_COUNT,
        ));
        functions.extend(
            metric_counts
                .iter()
                .copied()
                .enumerate()
                .map(|(index, count)| (ordinal(kind, index), count)),
        );
    }
    let mut report = format!("SF:{SOURCE}\n");
    for (name, _) in &functions {
        report.push_str(&format!("FN:{GROUP_LINE},{name}\n"));
    }
    for (name, count) in &functions {
        report.push_str(&format!("FNDA:{count},{name}\n"));
    }
    let hit = functions.iter().filter(|(_, count)| *count > 0).count();
    report.push_str(&format!(
        "FNF:{}\nFNH:{hit}\nend_of_record\n",
        functions.len()
    ));
    report
}

fn inventory_tokens() -> Result<Vec<String>, String> {
    let source = source_inventory_digest(&[SOURCE])?;
    let instance = instance_digest(GROUP_LINE, GROUP_COLUMN, DIGEST, DIGEST, &shapes());
    let inventory = instance_inventory_digest(&[&instance])?;
    Ok(vec![
        source_inventory_token(&source, PRESENCE_COUNT),
        instance_inventory_token(&inventory, PRESENCE_COUNT),
        instance_member_token(&inventory, &instance),
    ])
}

fn shapes() -> [MetricShape; 4] {
    [
        MetricShape {
            found: FULL_VECTOR_LENGTH,
            vector_len: FULL_VECTOR_LENGTH,
        },
        MetricShape {
            found: FUNCTION_VECTOR_LENGTH,
            vector_len: FUNCTION_VECTOR_LENGTH,
        },
        MetricShape {
            found: FULL_VECTOR_LENGTH,
            vector_len: FULL_VECTOR_LENGTH,
        },
        MetricShape {
            found: FULL_VECTOR_LENGTH,
            vector_len: FULL_VECTOR_LENGTH,
        },
    ]
}

fn ordinal(kind: MetricKind, value: usize) -> String {
    ordinal_token(
        kind,
        GROUP_LINE,
        GROUP_COLUMN,
        DIGEST,
        DIGEST,
        u64::try_from(value).unwrap(),
    )
}

fn argument(prefix: &str, path: &Path) -> String {
    format!("{prefix}{}", path.display())
}
