use crate::pre_exec_rlimit_callbacks as callbacks;
use crate::pre_exec_rlimit_contract as contract;
use crate::pre_exec_rlimit_syntax as syntax;
use std::collections::BTreeSet;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_syntax::rust_source::RustSource;

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !file.relative.ends_with(contract::RUST_SOURCE_SUFFIX) {
        return;
    }
    let source = RustSource::parse(&file.text);
    let callback_bodies = callbacks::bodies(&source);
    let forbidden = source
        .tokens
        .iter()
        .enumerate()
        .filter(|(index, _)| syntax::setrlimit_call(&source.tokens, *index))
        .filter(|(index, _)| callback_bodies.iter().any(|body| body.contains(index)))
        .map(|(index, _)| index)
        .collect::<BTreeSet<_>>();
    for index in forbidden {
        violations.push(Violation::line(
            &file.relative,
            source.tokens[index].line,
            contract::POLICY_MESSAGE,
        ));
    }
}
