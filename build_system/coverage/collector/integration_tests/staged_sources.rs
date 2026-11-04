use crate::constants::{COVERAGE_MANIFEST_ENV, DECLARATION_FILE_LIMIT};
use crate::entrypoint_fixture::*;

const MANIFEST_FILE: &str = "staged-sources.manifest";
const UNREMAPPED: &str = "unremapped declared production coverage source";
const MISSING_MANIFEST: &str = "staged coverage source requires Bazel's source manifest";
const INVALID_UTF8: &str = "invalid staged coverage source manifest UTF-8";

#[test]
fn staged_production_cannot_publish_a_partial_report_with_other_valid_dependencies() {
    for selected in [JSON_FILE, LCOV_FILE] {
        let mut fixture = Fixture::new();
        let path = fixture.root().join(selected);
        let report = std::fs::read_to_string(&path).unwrap();
        let staged = format!("{BAZEL_WITNESS_PREFIX}{FIXTURE_SOURCE}");
        let changed = if selected == JSON_FILE {
            let mut json: serde_json::Value = serde_json::from_str(&report).unwrap();
            let mut file = json["data"][0]["files"][0].clone();
            file["filename"] = staged.into();
            json["data"][0]["files"].as_array_mut().unwrap().push(file);
            json.to_string()
        } else {
            format!("{report}{}", report.replace(FIXTURE_SOURCE, &staged))
        };
        std::fs::write(&path, changed).unwrap();
        let manifest = fixture.root().join(MANIFEST_FILE);
        std::fs::write(&manifest, FIXTURE_SOURCE).unwrap();
        fixture.command.env(COVERAGE_MANIFEST_ENV, MANIFEST_FILE);
        fixture.fails(UNREMAPPED);
        assert!(!fixture.profile().exists());
    }
}

#[test]
fn zero_hit_file_and_function_only_aliases_are_guarded_after_path_normalization() {
    for function_only in [false, true] {
        for absolute in [false, true] {
            let mut fixture = Fixture::new();
            let path = fixture.root().join(JSON_FILE);
            let report = std::fs::read_to_string(&path).unwrap();
            let mut json: serde_json::Value = serde_json::from_str(&report).unwrap();
            let staged = format!("{BAZEL_WITNESS_PREFIX}{FIXTURE_SOURCE}");
            let staged = if absolute {
                fixture.root().join(staged).to_string_lossy().into_owned()
            } else {
                format!("./{staged}")
            };
            if function_only {
                let function = &mut json["data"][0]["functions"][0];
                function["filenames"] = serde_json::json!([staged]);
                for field in ["/count", "/regions/0/4", "/branches/0/4", "/branches/0/5"] {
                    *function.pointer_mut(field).unwrap() = 0.into();
                }
            } else {
                let file = &mut json["data"][0]["files"][0];
                file["filename"] = staged.into();
                for metric in ["lines", "functions", "regions", "branches"] {
                    file["summary"][metric]["covered"] = 0.into();
                }
            }
            std::fs::write(path, json.to_string()).unwrap();
            std::fs::write(fixture.root().join(MANIFEST_FILE), FIXTURE_SOURCE).unwrap();
            fixture.command.env(COVERAGE_MANIFEST_ENV, MANIFEST_FILE);
            fixture.fails(UNREMAPPED);
            assert!(!fixture.profile().exists());
        }
    }
}

#[test]
fn absent_unreadable_invalid_and_oversized_staged_manifests_remain_fail_closed() {
    for scenario in ["absent", "missing", "utf8", "oversized", "malformed"] {
        let mut fixture = Fixture::new();
        let path = fixture.root().join(LCOV_FILE);
        let report = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            &path,
            report.replace(
                FIXTURE_SOURCE,
                &format!("{BAZEL_WITNESS_PREFIX}{FIXTURE_SOURCE}"),
            ),
        )
        .unwrap();
        let manifest = fixture.root().join(MANIFEST_FILE);
        if scenario != "absent" {
            fixture.command.env(COVERAGE_MANIFEST_ENV, &manifest);
        }
        let error = match scenario {
            "absent" => MISSING_MANIFEST,
            "missing" => "cannot read declaration evidence",
            "utf8" => {
                std::fs::write(&manifest, [u8::MAX]).unwrap();
                INVALID_UTF8
            }
            "oversized" => {
                std::fs::File::create(&manifest)
                    .unwrap()
                    .set_len(DECLARATION_FILE_LIMIT + 1)
                    .unwrap();
                "declaration evidence exceeds its size bound"
            }
            _ => {
                std::fs::write(&manifest, "../invalid.rs").unwrap();
                "invalid staged coverage source manifest"
            }
        };
        fixture.fails(error);
    }
}
