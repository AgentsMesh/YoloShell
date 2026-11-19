use crate::constants::{CROSS_CRATE_PATH_MESSAGE, PATH_ATTRIBUTE_NAME};
use crate::paths;
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::{rust_source, targeted_tokens::Kind};

const PATH_ATTRIBUTE_TOKEN_COUNT: usize = 6;
const MARKER_INDEX: usize = 0;
const OPEN_INDEX: usize = 1;
const NAME_INDEX: usize = 2;
const ASSIGNMENT_INDEX: usize = 3;
const VALUE_INDEX: usize = 4;
const CLOSE_INDEX: usize = 5;

pub fn inspect(
    files: &[SourceFile],
    repository: &crate::Repository,
    production_sources: &std::collections::BTreeSet<String>,
    violations: &mut Vec<Violation>,
) {
    for file in files
        .iter()
        .filter(|file| production_sources.contains(&file.relative))
    {
        let source = rust_source::RustSource::parse(&file.text);
        for tokens in source.tokens.windows(PATH_ATTRIBUTE_TOKEN_COUNT) {
            if !path_attribute(tokens) {
                continue;
            }
            let Kind::String(relative) = &tokens[VALUE_INDEX].kind else {
                continue;
            };
            let Some(target) = paths::resolve_relative(&file.relative, relative) else {
                continue;
            };
            if !production_sources.contains(&target) {
                continue;
            }
            let source_package = repository.source_package(&file.relative);
            let target_package = repository.source_package(&target);
            if source_package != target_package {
                violations.push(Violation::line(
                    &file.relative,
                    tokens[MARKER_INDEX].line,
                    CROSS_CRATE_PATH_MESSAGE,
                ));
            }
        }
    }
}

fn path_attribute(tokens: &[yolo_architecture_syntax::targeted_tokens::Token]) -> bool {
    tokens[MARKER_INDEX].symbol("#")
        && tokens[OPEN_INDEX].symbol("[")
        && tokens[NAME_INDEX].identifier(PATH_ATTRIBUTE_NAME)
        && tokens[ASSIGNMENT_INDEX].symbol("=")
        && tokens[CLOSE_INDEX].symbol("]")
}
