#![allow(dead_code)]

pub const VARIANTS: &[&str] = &[
    "YOLO_CANCELLATION_O0_DEFAULT",
    "YOLO_CANCELLATION_O0_SINGLE",
    "YOLO_CANCELLATION_O3_DEFAULT",
    "YOLO_CANCELLATION_O3_SINGLE",
    "YOLO_CANCELLATION_O3_THIN_LTO",
    "YOLO_CANCELLATION_O3_FAT_LTO",
];
pub const TOOLS_ENV: &str = "YOLO_COVERAGE_PROBE_TOOLS";
pub const WORKSPACE_ENV: &str = "TEST_WORKSPACE";
pub const PROFILE_ENV: &str = "LLVM_PROFILE_FILE";
pub const PROFILE_PATTERN: &str = "%h-%p-%m.profraw%c";
pub const PROFILE_EXTENSION: &str = "profraw";
pub const MERGED_FILE: &str = "probe.profdata";
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
pub const MAX_OUTPUT: u64 = 8 * 1024 * 1024;
pub const WITNESS: &str = "await_started=1; await_resumed=0; drop_entered=1; drop_returned=0\n";
pub const ZERO_FUNCTIONS: &[&str] = &["yolo_probe_after_await", "yolo_probe_after_drop"];
pub const DATA: &str = "data";
pub const FUNCTIONS: &str = "functions";
pub const NAME: &str = "name";
pub const COUNT: &str = "count";
pub const REGIONS: &str = "regions";
pub const BRANCHES: &str = "branches";
pub const FILENAMES: &str = "filenames";
pub const SOURCE: &str = "build_system/coverage/compiler/test_support/cancellation_probe.rs";
pub const SOURCE_TEXT: &str = include_str!("cancellation_probe.rs");
pub const ASYNC_ENTRY: &str = "STARTED.fetch_add(ONCE, Ordering::SeqCst);";
pub const CALLER_ENTRY: &str = "drop(guard);";
pub const DROP_ENTRY: &str = "DROP_ENTERED.fetch_add(ONCE, Ordering::SeqCst);";
pub const MAIN_ENTRY: &str = "std::panic::set_hook(Box::new(|_| {}));";
pub const AFTER_AWAIT: &str = "yolo_probe_after_await();";
pub const AFTER_DROP: &str = "yolo_probe_after_drop();";
pub const DROP_CONDITION: &str = "std::hint::black_box(true)";
pub const START_LINE_INDEX: usize = 0;
pub const START_COLUMN_INDEX: usize = 1;
pub const END_LINE_INDEX: usize = 2;
pub const END_COLUMN_INDEX: usize = 3;
pub const REGION_FILE_ID_INDEX: usize = 5;
pub const REGION_KIND_INDEX: usize = 7;
pub const BRANCH_FILE_ID_INDEX: usize = 6;
pub const BRANCH_KIND_INDEX: usize = 8;
pub const CODE_REGION_KIND: u64 = 0;
pub const BRANCH_REGION_KIND: u64 = 4;
pub const COUNT_INDEX: usize = 4;
pub const FALSE_COUNT_INDEX: usize = 5;
pub const SATURATED: u64 = i64::MAX as u64;
pub const ZERO: u64 = 0;
pub const ONCE: u64 = 1;
pub const SINGLE: usize = 1;
pub const LLVM_PROFDATA: &str = "llvm_profdata";
pub const LLVM_COV: &str = "llvm_cov";
pub const MERGE: &str = "merge";
pub const SPARSE: &str = "--sparse";
pub const OUTPUT: &str = "--output";
pub const EXPORT: &str = "export";
pub const PROFILE_ARGUMENT: &str = "--instr-profile";
