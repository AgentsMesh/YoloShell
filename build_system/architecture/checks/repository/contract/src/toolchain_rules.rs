use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_foundation::text::quoted_literals;

const BAZEL_MODULE: &str = "MODULE.bazel";
const BAZEL_RC: &str = ".bazelrc";
const CARGO_VERSION_KEY: &str = "rust-version";
const BAZEL_TOOLCHAIN_CALL: &str = "rust.toolchain(";
const BAZEL_VERSIONS_KEY: &str = "versions";
const FIRST_SOURCE_LINE: usize = 1;
const PINNED_VERSION_COMPONENTS: usize = 3;
pub const RUST_COVERAGE_DECLARATIONS: &[&str] = &[
    "coverage:rust_tooling --config=rust",
    "coverage:rust_tooling --instrumentation_filter=^//(core|plugins|build_system|integration_tests)[/:]",
    "coverage:rust --instrumentation_filter=^//(core|plugins|integration_tests)[/:]",
    "coverage:rust --@rules_rust//rust/settings:extra_rustc_env=RUSTC_BOOTSTRAP=1",
    "coverage:rust --@rules_rust//rust/settings:extra_rustc_flag=-Zcoverage-options=branch",
];

#[derive(Default)]
struct Declaration {
    line: Option<usize>,
    values: Vec<String>,
}

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    inspect_coverage_configuration(files, violations);
    let Some(cargo) = file(
        files,
        yolo_architecture_policy_contracts::constants::CARGO_MANIFEST_PATH,
    ) else {
        return;
    };
    let Some(module) = file(files, BAZEL_MODULE) else {
        return;
    };
    let cargo_version = cargo_declaration(&cargo.text);
    let bazel_version = bazel_declaration(&module.text);
    let cargo_valid = validate(cargo, &cargo_version, violations);
    let bazel_valid = validate(module, &bazel_version, violations);
    if cargo_valid && bazel_valid && cargo_version.values != bazel_version.values {
        violations.push(Violation::line(
            &cargo.relative,
            cargo_version.line.unwrap_or(FIRST_SOURCE_LINE),
            format!(
                "Cargo Rust version {} differs from Bazel toolchain {}",
                cargo_version.values[0], bazel_version.values[0]
            ),
        ));
    }
}

fn inspect_coverage_configuration(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let Some(configuration) = file(files, BAZEL_RC) else {
        return;
    };
    for expected in RUST_COVERAGE_DECLARATIONS {
        let matches = configuration
            .text
            .lines()
            .filter(|line| line.trim() == *expected)
            .count();
        if matches != 1 {
            violations.push(Violation::line(
                &configuration.relative,
                FIRST_SOURCE_LINE,
                format!("Rust coverage configuration must contain exactly once: {expected}"),
            ));
        }
    }
}

fn file<'a>(files: &'a [SourceFile], relative: &str) -> Option<&'a SourceFile> {
    files.iter().find(|file| file.relative == relative)
}

fn cargo_declaration(text: &str) -> Declaration {
    let mut result = Declaration::default();
    for (index, line) in text.lines().enumerate() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != CARGO_VERSION_KEY {
            continue;
        }
        result.line.get_or_insert(index + FIRST_SOURCE_LINE);
        result
            .values
            .extend(quoted_literals(value).into_iter().map(|item| item.value));
    }
    result
}

fn bazel_declaration(text: &str) -> Declaration {
    let mut result = Declaration::default();
    let mut in_toolchain = false;
    let mut in_versions = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if !in_toolchain {
            in_toolchain = trimmed == BAZEL_TOOLCHAIN_CALL;
            continue;
        }
        if !in_versions && assignment_key(trimmed) == Some(BAZEL_VERSIONS_KEY) {
            result.line.get_or_insert(index + FIRST_SOURCE_LINE);
            in_versions = true;
        }
        if in_versions {
            result
                .values
                .extend(quoted_literals(line).into_iter().map(|item| item.value));
            in_versions = !trimmed.contains(']');
        }
        if !in_versions && (trimmed == ")" || trimmed == "),") {
            in_toolchain = false;
        }
    }
    result
}

fn assignment_key(line: &str) -> Option<&str> {
    line.split_once('=').map(|(key, _)| key.trim())
}

fn validate(file: &SourceFile, declaration: &Declaration, violations: &mut Vec<Violation>) -> bool {
    if declaration.values.len() != 1 || !pinned_version(&declaration.values[0]) {
        violations.push(Violation::line(
            &file.relative,
            declaration.line.unwrap_or(FIRST_SOURCE_LINE),
            "Rust toolchain must declare exactly one major.minor.patch stable version",
        ));
        return false;
    }
    true
}

fn pinned_version(value: &str) -> bool {
    let components = value.split('.').collect::<Vec<_>>();
    components.len() == PINNED_VERSION_COMPONENTS
        && components.iter().all(|component| {
            !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
        })
}
