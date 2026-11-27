use super::recipe::{document, file};
use crate::recipe::read;
use tempfile::TempDir;

#[test]
fn wait_edges_require_named_dependencies_and_bounded_timeouts() {
    use crate::recipe::{RecipeWait, RecipeWaitUntil};
    let directory = TempDir::new().unwrap();
    let mut document = document();
    document.agents[1].after = vec!["planner".into()];
    document.agents[1].wait = Some(RecipeWait {
        until: RecipeWaitUntil::Exited,
        timeout_seconds: crate::constants::DEFAULT_RECIPE_WAIT_SECONDS,
    });
    let parsed = read(&file(&directory, &document)).unwrap();
    assert_eq!(
        parsed.agents[1].wait.map(|wait| wait.until),
        Some(RecipeWaitUntil::Exited)
    );

    let mut orphan = document.clone();
    orphan.agents[0].wait = Some(RecipeWait {
        until: RecipeWaitUntil::Promptable,
        timeout_seconds: crate::constants::DEFAULT_RECIPE_WAIT_SECONDS,
    });
    assert!(read(&file(&directory, &orphan)).is_err());

    for timeout_seconds in [0, crate::constants::MAX_RECIPE_WAIT_SECONDS + 1] {
        let mut bounded = document.clone();
        bounded.agents[1].wait = Some(RecipeWait {
            until: RecipeWaitUntil::Idle,
            timeout_seconds,
        });
        assert!(read(&file(&directory, &bounded)).is_err());
    }
}

#[test]
fn prompts_are_bounded_and_independent_of_dependencies() {
    use crate::recipe::{RecipePrompt, RecipePromptWait};
    let directory = TempDir::new().unwrap();
    let mut document = document();
    document.agents[0].prompt = Some(RecipePrompt {
        text: "Plan the work".into(),
        timeout_seconds: crate::constants::DEFAULT_RECIPE_PROMPT_SECONDS,
        wait: RecipePromptWait::TurnEnd,
    });
    let parsed = read(&file(&directory, &document)).unwrap();
    assert_eq!(
        parsed.agents[0]
            .prompt
            .as_ref()
            .map(|prompt| prompt.text.as_str()),
        Some("Plan the work")
    );
    assert_eq!(
        parsed.agents[0].prompt.as_ref().map(|prompt| prompt.wait),
        Some(RecipePromptWait::TurnEnd)
    );

    let raw = serde_json::json!({
        (crate::constants::SCHEMA_VERSION_FIELD): crate::constants::RECIPE_SCHEMA_VERSION,
        "workspace": document.workspace,
        "agents": [{"provider": "codex", "name": "planner", "prompt": {"text": "Plan"}}],
    });
    let path = directory.path().join("default-wait.json");
    std::fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
    let parsed = read(&path).unwrap();
    assert_eq!(
        parsed.agents[0].prompt.as_ref().map(|prompt| prompt.wait),
        Some(RecipePromptWait::Delivered)
    );

    for invalid in ["", "   \n\t"] {
        let mut blank = document.clone();
        blank.agents[0].prompt = Some(RecipePrompt {
            text: invalid.into(),
            timeout_seconds: crate::constants::DEFAULT_RECIPE_PROMPT_SECONDS,
            wait: RecipePromptWait::Delivered,
        });
        assert!(read(&file(&directory, &blank)).is_err());
    }
    for timeout_seconds in [0, crate::constants::MAX_RECIPE_PROMPT_SECONDS + 1] {
        let mut bounded = document.clone();
        bounded.agents[0].prompt = Some(RecipePrompt {
            text: "Plan".into(),
            timeout_seconds,
            wait: RecipePromptWait::Delivered,
        });
        assert!(read(&file(&directory, &bounded)).is_err());
    }
}

#[test]
fn prompt_edges_accept_unnamed_providers_and_named_dependencies() {
    let directory = TempDir::new().unwrap();
    let mut document = document();
    document.agents[0].prompt = Some(crate::recipe::RecipePrompt {
        text: "Plan".into(),
        timeout_seconds: crate::constants::DEFAULT_RECIPE_PROMPT_SECONDS,
        wait: crate::recipe::RecipePromptWait::Delivered,
    });
    document.agents[1].after = vec!["planner".into()];
    document.agents[1].wait = Some(crate::recipe::RecipeWait {
        until: crate::recipe::RecipeWaitUntil::Promptable,
        timeout_seconds: crate::constants::DEFAULT_RECIPE_WAIT_SECONDS,
    });
    assert!(read(&file(&directory, &document)).is_ok());
}
