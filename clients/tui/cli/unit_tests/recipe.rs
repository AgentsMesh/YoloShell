use crate::recipe::{RecipeAgent, RecipeDocument, read};
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

pub(super) fn agent(provider: &str, name: Option<&str>) -> RecipeAgent {
    RecipeAgent {
        provider: provider.into(),
        name: name.map(str::to_owned),
        start_token: None,
        labels: Default::default(),
        after: Vec::new(),
        wait: None,
        prompt: None,
    }
}

pub(super) fn document() -> RecipeDocument {
    RecipeDocument {
        schema_version: crate::constants::RECIPE_SCHEMA_VERSION,
        workspace: Some(Uuid::new_v4()),
        cwd: None,
        parent: None,
        stop_on_failure: false,
        labels: Default::default(),
        agents: vec![
            agent("codex", Some("planner")),
            agent("codex", Some("worker")),
        ],
    }
}

pub(super) fn file(directory: &TempDir, document: &RecipeDocument) -> PathBuf {
    let path = directory.path().join("recipe.json");
    std::fs::write(&path, serde_json::to_vec(document).unwrap()).unwrap();
    path
}

#[test]
fn stop_on_failure_is_optional_and_defaults_to_keep() {
    let directory = TempDir::new().unwrap();
    let mut document = document();
    document.stop_on_failure = true;
    let parsed = read(&file(&directory, &document)).unwrap();
    assert!(parsed.stop_on_failure);
    let mut without = document;
    without.stop_on_failure = false;
    let parsed = read(&file(&directory, &without)).unwrap();
    assert!(!parsed.stop_on_failure);
}

#[test]
fn dependencies_produce_a_stable_topological_order() {
    use crate::recipe::compute;
    let mut document = document();
    document.agents = vec![
        agent("codex", Some("second")),
        agent("codex", Some("first")),
    ];
    document.agents[0].after = vec!["first".into()];
    assert_eq!(compute(&document).unwrap(), vec![1, 0]);

    let mut cycle = document.clone();
    cycle.agents[1].after = vec!["second".into()];
    assert!(compute(&cycle).is_err());

    let mut unknown = document.clone();
    unknown.agents[0].after = vec!["absent".into()];
    assert!(compute(&unknown).is_err());

    let mut duplicate = document.clone();
    duplicate.agents[0].after = vec!["first".into(), "FIRST".into()];
    assert!(compute(&duplicate).is_err());

    let mut own = document;
    own.agents[1].after = vec!["first".into()];
    assert!(compute(&own).is_err());
}

#[test]
fn labels_merge_with_agent_precedence_and_are_bounded() {
    use crate::recipe::merged;
    let mut document_labels = std::collections::BTreeMap::new();
    document_labels.insert("team".to_string(), "alpha".to_string());
    document_labels.insert("role".to_string(), "worker".to_string());
    let mut agent_labels = std::collections::BTreeMap::new();
    agent_labels.insert("role".to_string(), "planner".to_string());
    let merged = merged(&document_labels, &agent_labels);
    assert_eq!(merged["team"], "alpha");
    assert_eq!(merged["role"], "planner");

    let directory = TempDir::new().unwrap();
    let mut invalid = document();
    invalid.agents[0]
        .labels
        .insert(String::new(), "value".into());
    assert!(read(&file(&directory, &invalid)).is_err());

    let mut too_many = document();
    for index in 0..=yolo_hub_agent_run_protocol::MAX_AGENT_RUN_LABELS {
        too_many.labels.insert(format!("k{index}"), "v".into());
    }
    assert!(read(&file(&directory, &too_many)).is_err());
}

#[test]
fn a_valid_recipe_round_trips_in_order() {
    let directory = TempDir::new().unwrap();
    let document = document();
    let parsed = read(&file(&directory, &document)).unwrap();
    assert_eq!(parsed.agents.len(), 2);
    assert_eq!(parsed.agents[0].name.as_deref(), Some("planner"));
    assert_eq!(parsed.agents[1].name.as_deref(), Some("worker"));
    assert_eq!(parsed.workspace, document.workspace);
}

#[test]
fn schema_bounds_and_required_location_are_enforced() {
    let directory = TempDir::new().unwrap();
    let mut wrong_schema = document();
    wrong_schema.schema_version = 2;
    assert!(read(&file(&directory, &wrong_schema)).is_err());

    let mut empty = document();
    empty.agents.clear();
    assert!(read(&file(&directory, &empty)).is_err());

    let mut many = document();
    many.agents = (0..=crate::constants::MAX_RECIPE_AGENTS)
        .map(|index| agent("codex", Some(&format!("a{index}"))))
        .collect();
    assert!(read(&file(&directory, &many)).is_err());

    let mut nowhere = document();
    nowhere.workspace = None;
    nowhere.cwd = None;
    assert!(read(&file(&directory, &nowhere)).is_err());

    let mut relative = document();
    relative.cwd = Some("relative/path".into());
    assert!(read(&file(&directory, &relative)).is_err());
}

#[test]
fn agents_are_uniquely_named_and_tokens_are_not_nil() {
    let directory = TempDir::new().unwrap();
    let mut duplicate = document();
    duplicate.agents = vec![agent("codex", Some("same")), agent("codex", Some("SAME"))];
    assert!(read(&file(&directory, &duplicate)).is_err());

    let mut nil = document();
    nil.agents[0].start_token = Some(Uuid::nil());
    assert!(read(&file(&directory, &nil)).is_err());

    let mut empty_provider = document();
    empty_provider.agents[0].provider.clear();
    assert!(read(&file(&directory, &empty_provider)).is_err());
}
