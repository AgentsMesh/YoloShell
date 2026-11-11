use super::test_support::{self, Fixture};
use std::fs;

const TARGET_FILE: &str = "target.jsonl";
const HARD_LINK_FILE: &str = "hard-link.jsonl";
const DISPLACED_DIRECTORY: &str = "displaced-day";
const DISPLACED_HOME: &str = "displaced-codex-home";

#[cfg(unix)]
#[test]
fn symbolic_and_hard_link_rollouts_are_rejected() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    let target = fixture.rollout_directory.join(TARGET_FILE);
    fs::copy(&evidence, &target).unwrap();
    fs::remove_file(&evidence).unwrap();
    symlink(&target, &evidence).unwrap();
    assert!(fixture.inspect(&evidence).is_none());

    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    fs::hard_link(&evidence, fixture.rollout_directory.join(HARD_LINK_FILE)).unwrap();
    assert!(fixture.inspect(&evidence).is_none());
}

#[cfg(unix)]
#[test]
fn symbolic_directory_component_and_data_home_are_rejected() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    let displaced = fixture
        .rollout_directory
        .parent()
        .unwrap()
        .join(DISPLACED_DIRECTORY);
    fs::rename(&fixture.rollout_directory, &displaced).unwrap();
    symlink(&displaced, &fixture.rollout_directory).unwrap();
    assert!(fixture.inspect(&evidence).is_none());

    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    let displaced = fixture.data_home.with_file_name(DISPLACED_HOME);
    fs::rename(&fixture.data_home, &displaced).unwrap();
    symlink(&displaced, &fixture.data_home).unwrap();
    assert!(fixture.inspect(&evidence).is_none());
}

#[test]
fn directory_and_root_replacement_after_read_are_rejected() {
    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    let contents = fs::read(&evidence).unwrap();
    let displaced = fixture
        .rollout_directory
        .parent()
        .unwrap()
        .join(DISPLACED_DIRECTORY);
    let result = fixture.inspect_with_hook(&evidence, || {
        fs::rename(&fixture.rollout_directory, &displaced).unwrap();
        fs::create_dir(&fixture.rollout_directory).unwrap();
        fs::write(&evidence, &contents).unwrap();
    });
    assert!(result.is_none());

    let fixture = Fixture::new();
    let evidence = fixture.write_main(test_support::SESSION_ID);
    let contents = fs::read(&evidence).unwrap();
    let displaced = fixture.data_home.with_file_name(DISPLACED_HOME);
    let result = fixture.inspect_with_hook(&evidence, || {
        fs::rename(&fixture.data_home, &displaced).unwrap();
        fs::create_dir_all(evidence.parent().unwrap()).unwrap();
        fs::write(&evidence, contents).unwrap();
    });
    assert!(result.is_none());
}
