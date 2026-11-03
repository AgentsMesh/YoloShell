use crate::coverage_json::*;
use crate::instance_fixture::*;
use crate::instance_geometry::parse;
use std::path::Path;

#[test]
fn normalized_main_geometry_retains_exact_positions_and_regions() {
    let mut function = function();
    function.filenames[FILE_INDEX as usize] = format!("{ROOT}/{SOURCE}");
    let geometry = parse(&function, Path::new(ROOT)).unwrap().unwrap();
    assert_eq!(geometry.source, SOURCE);
    assert_eq!(geometry.filenames, [SOURCE]);
    assert_eq!(geometry.line, POSITION);
    assert_eq!(geometry.column, POSITION);
    assert_eq!(geometry.main_regions.len(), SINGLE_METRIC as usize);
}

#[test]
fn test_sources_are_excluded_without_creating_production_geometry() {
    let mut function = function();
    function.filenames[FILE_INDEX as usize] = TEST_SOURCE.to_owned();
    assert!(parse(&function, Path::new(ROOT)).unwrap().is_none());
}

#[test]
fn names_filenames_and_main_regions_are_required() {
    for invalid in [
        |function: &mut Function| function.name.clear(),
        |function: &mut Function| function.filenames.clear(),
        |function: &mut Function| function.filenames[FILE_INDEX as usize].clear(),
        |function: &mut Function| function.regions.clear(),
    ] {
        let mut function = function();
        invalid(&mut function);
        assert!(parse(&function, Path::new(ROOT)).is_err());
    }
}

#[test]
fn expansion_cannot_reference_outside_the_file_table_or_remove_every_main_file() {
    for expanded in [FILE_INDEX, OUTSIDE_FILE_INDEX] {
        let mut function = function();
        function.regions[FILE_INDEX as usize][KIND_INDEX] = EXPANSION_REGION_KIND;
        function.regions[FILE_INDEX as usize][EXPANDED_FILE_ID_INDEX] = expanded;
        assert!(parse(&function, Path::new(ROOT)).is_err());
    }
    let mut function = function();
    function.filenames.push(OTHER_SOURCE.to_owned());
    let mut expansion = function.regions[FILE_INDEX as usize];
    expansion[KIND_INDEX] = EXPANSION_REGION_KIND;
    expansion[EXPANDED_FILE_ID_INDEX] = SINGLE_METRIC;
    function.regions.push(expansion);
    assert_eq!(
        parse(&function, Path::new(ROOT)).unwrap().unwrap().source,
        SOURCE
    );
}

#[test]
fn region_and_branch_file_references_are_checked() {
    let mut function = function();
    function.regions[FILE_INDEX as usize][FILE_ID_INDEX] = OUTSIDE_FILE_INDEX;
    assert!(parse(&function, Path::new(ROOT)).is_err());
    for index in [BRANCH_FILE_ID_INDEX, BRANCH_EXPANDED_FILE_ID_INDEX] {
        let mut function = crate::instance_fixture::function();
        function.branches[FILE_INDEX as usize][index] = OUTSIDE_FILE_INDEX;
        assert!(parse(&function, Path::new(ROOT)).is_err());
    }
}

#[test]
fn invalid_coordinates_and_region_kinds_are_rejected() {
    for value in [u64::default(), u64::from(u32::MAX) + SINGLE_METRIC] {
        for index in [
            LINE_START_INDEX,
            COLUMN_START_INDEX,
            LINE_END_INDEX,
            COLUMN_END_INDEX,
        ] {
            let mut function = function();
            function.regions[FILE_INDEX as usize][index] = value;
            assert!(parse(&function, Path::new(ROOT)).is_err());
            let mut function = crate::instance_fixture::function();
            function.branches[FILE_INDEX as usize][index] = value;
            assert!(parse(&function, Path::new(ROOT)).is_err());
        }
    }
    let mut function = function();
    function.regions[FILE_INDEX as usize][KIND_INDEX] = u64::MAX;
    assert!(parse(&function, Path::new(ROOT)).is_err());
}

#[test]
fn another_file_cannot_supply_the_primary_sources_missing_region() {
    let mut function = function();
    function.filenames.push(OTHER_SOURCE.to_owned());
    function.regions[FILE_INDEX as usize][FILE_ID_INDEX] = SINGLE_METRIC;
    let error = match parse(&function, Path::new(ROOT)) {
        Ok(_) => panic!("a different source supplied the missing main-file region"),
        Err(error) => error,
    };
    assert!(error.contains("has no main-file region"), "{error}");
}

#[test]
fn reversed_source_ranges_cannot_become_coverage_evidence() {
    for branch in [false, true] {
        let mut function = function();
        if branch {
            function.branches[FILE_INDEX as usize][COLUMN_START_INDEX] = END_COLUMN + SINGLE_METRIC;
        } else {
            function.regions[FILE_INDEX as usize][COLUMN_START_INDEX] = END_COLUMN + SINGLE_METRIC;
        }
        let error = match parse(&function, Path::new(ROOT)) {
            Ok(_) => panic!("a reversed source range was accepted"),
            Err(error) => error,
        };
        assert_eq!(error, "invalid LLVM coverage map coordinates");
    }
}
