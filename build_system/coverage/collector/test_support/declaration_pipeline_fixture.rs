use crate::constants::COVERAGE_MANIFEST_ENV;
use crate::entrypoint_fixture::*;
use crate::instance_fixture::{FILE_INDEX, TEST_SOURCE};
use std::path::PathBuf;
use yolo_coverage_source_contract::DECLARATION_SOURCE_SETS;

pub const MANIFEST_FILE: &str = "declaration-sources.manifest";
pub const UNDECLARED_SOURCE_CONTENT: &[u8] = b"pub fn unexpected_runtime_code() {}";
pub const INVALID_UTF8: &[u8] = &[0xff];

pub fn prepare() -> Fixture {
    let mut fixture = Fixture::new();
    let sources = DECLARATION_SOURCE_SETS[FILE_INDEX as usize];
    let root = source_root(&fixture);
    for (source, _) in sources {
        let target = root.join(source);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(source, target).unwrap();
    }
    std::fs::write(
        fixture.root().join(MANIFEST_FILE),
        crate::declaration_fixture::manifest(sources),
    )
    .unwrap();
    let json_path = fixture.root().join(JSON_FILE);
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&json_path).unwrap()).unwrap();
    let data = &mut json["data"][FILE_INDEX as usize];
    data["files"][FILE_INDEX as usize]["filename"] = TEST_SOURCE.into();
    data["functions"][FILE_INDEX as usize]["filenames"][FILE_INDEX as usize] = TEST_SOURCE.into();
    std::fs::write(json_path, serde_json::to_vec(&json).unwrap()).unwrap();
    let lcov_path = fixture.root().join(LCOV_FILE);
    let text = std::fs::read_to_string(&lcov_path).unwrap();
    std::fs::write(lcov_path, text.replace(FIXTURE_SOURCE, TEST_SOURCE)).unwrap();
    fixture.command.env(COVERAGE_MANIFEST_ENV, MANIFEST_FILE);
    fixture
}

pub fn source_root(fixture: &Fixture) -> PathBuf {
    fixture.root().join(FIXTURE_RUNFILES).join(TEST_WORKSPACE)
}

pub fn first_source(fixture: &Fixture) -> PathBuf {
    source_root(fixture).join(DECLARATION_SOURCE_SETS[FILE_INDEX as usize][FILE_INDEX as usize].0)
}

pub fn prepare_test_only(source: &str, support: &str) -> Fixture {
    let fixture = prepare();
    std::fs::write(
        fixture.root().join(MANIFEST_FILE),
        format!("{source}\n{support}"),
    )
    .unwrap();
    for name in [JSON_FILE, LCOV_FILE] {
        let path = fixture.root().join(name);
        let original = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, original.replace(TEST_SOURCE, source)).unwrap();
    }
    fixture
}
