use crate::lcov_fixture::{self, Decorations, Entry};
use yolo_coverage_protocol::{MetricKind, header_token, ordinal_token};

pub const CORE_SOURCE: &str = "core/crates/wire/src/lib.rs";
pub const TOOLING_SOURCE: &str = "build_system/tool/src/lib.rs";
pub const SHARED_SOURCES: [&str; 2] = [TOOLING_SOURCE, CORE_SOURCE];
pub const SINGLE_INSTANCE: usize = 1;
pub const LARGE_INSTANCE_COUNT: usize = 100;
const IDENTITY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SIGNATURE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const FIRST_POSITION: u64 = 1;
const SINGLE_ORDINAL: u64 = 1;
const FIRST_ORDINAL: u64 = 0;
const PRESENCE_COUNT: u64 = 1;

pub fn entries(count: usize, covered: bool) -> Vec<Entry> {
    (0..count)
        .flat_map(|index| {
            let line = u64::try_from(index).unwrap() + FIRST_POSITION;
            MetricKind::ALL.into_iter().flat_map(move |kind| {
                [
                    (
                        line,
                        header_token(
                            kind,
                            line,
                            FIRST_POSITION,
                            IDENTITY,
                            SIGNATURE,
                            SINGLE_ORDINAL,
                            SINGLE_ORDINAL,
                        ),
                        PRESENCE_COUNT,
                    ),
                    (
                        line,
                        ordinal_token(
                            kind,
                            line,
                            FIRST_POSITION,
                            IDENTITY,
                            SIGNATURE,
                            FIRST_ORDINAL,
                        ),
                        u64::from(covered),
                    ),
                ]
            })
        })
        .collect()
}

pub fn report(source: &str, count: usize, covered: bool) -> String {
    let entries = entries(count, covered);
    lcov_fixture::report(source, &entries, &entries, Decorations::Complete)
}

pub fn shared_report(source: &str, covered: bool) -> String {
    let entries = entries(SINGLE_INSTANCE, covered);
    lcov_fixture::report_with_sources(
        source,
        &entries,
        &entries,
        Decorations::Complete,
        &SHARED_SOURCES,
    )
}
