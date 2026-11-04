#[path = "../test_support/constants.rs"]
mod constants;
#[path = "../test_support/resources.rs"]
mod resources;
use constants::*;

#[test]
fn darwin_mapping_sections_have_independent_segments_and_page_alignment() {
    let settings = resources::settings();
    let commands = [
        vec![
            SECTION_RENAME,
            DATA_SEGMENT,
            COUNTER_SECTION,
            COUNTER_SEGMENT,
            COUNTER_SECTION,
        ],
        vec![
            SECTION_RENAME,
            DATA_SEGMENT,
            BITMAP_SECTION,
            BITMAP_SEGMENT,
            BITMAP_SECTION,
        ],
        vec![
            SECTION_ALIGNMENT,
            COUNTER_SEGMENT,
            COUNTER_SECTION,
            PAGE_ALIGNMENT,
        ],
        vec![
            SECTION_ALIGNMENT,
            BITMAP_SEGMENT,
            BITMAP_SECTION,
            PAGE_ALIGNMENT,
        ],
        vec![
            SECTION_ALIGNMENT,
            DATA_SEGMENT,
            PROFILE_DATA_SECTION,
            PAGE_ALIGNMENT,
        ],
    ];
    let expected = commands
        .iter()
        .map(|arguments| format!("{DRIVER_LINK_ARGUMENT}{}", arguments.join(",")))
        .collect::<Vec<_>>();
    assert_eq!(settings[MACOS_FLAGS_FIELD], serde_json::json!(expected));
    let direct = commands
        .iter()
        .flatten()
        .map(|argument| format!("{DIRECT_LINK_ARGUMENT}{argument}"))
        .collect::<Vec<_>>();
    assert_eq!(
        settings[DIRECT_MACOS_FLAGS_FIELD],
        serde_json::json!(direct)
    );
    assert_eq!(
        settings[MACOS_ENV_FIELD],
        serde_json::json!({ CONTINUOUS_ENV: ENABLED })
    );
}

#[test]
fn non_darwin_toolchains_keep_their_existing_flags_and_environment() {
    let settings = resources::settings();
    for flags in settings[OTHER_FLAGS_FIELD].as_object().unwrap().values() {
        assert!(flags.as_array().unwrap().is_empty());
    }
    for environment in settings[OTHER_ENV_FIELD].as_object().unwrap().values() {
        assert!(environment.as_object().unwrap().is_empty());
    }
}

#[test]
fn fixture_source_never_contributes_to_production_coverage() {
    for source in [PROBE_SOURCE, OPTIMIZED_PROBE_SOURCE, MCDC_PROBE_SOURCE] {
        assert!(
            yolo_coverage_protocol::production_key(source)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn actual_test_environment_matches_the_bazel_coverage_mode() {
    let enabled = resources::settings()[COVERAGE_FIELD].as_bool().unwrap();
    let mode = std::env::var(CONTINUOUS_ENV).ok();
    if enabled {
        assert_eq!(mode.as_deref(), Some(ENABLED));
        assert!(
            std::env::var(PROFILE_ENV)
                .unwrap()
                .contains(CONTINUOUS_MARKER)
        );
    } else {
        assert!(mode.is_none());
    }
}
