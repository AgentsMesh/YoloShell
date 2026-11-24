use crate::pty_master_closure_contract as contract;
use crate::pty_master_closure_sites as sites;
use crate::pty_master_closure_syntax as syntax;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::Token;

struct Seen {
    methods: Vec<usize>,
    constructors: Vec<usize>,
}

pub(super) fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    if source(files, contract::SETTLEMENT_PATH).is_none() {
        return;
    }
    let mut seen = Seen {
        methods: zeroed(contract::METHOD_SITES),
        constructors: zeroed(contract::CONSTRUCTOR_SITES),
    };
    for file in files.iter().filter(|file| production_rust(file)) {
        inspect_file(file, &mut seen, violations);
    }
    if incomplete(&seen)
        && let Some(owner) = source(files, contract::SETTLEMENT_PATH)
    {
        violations.push(Violation::file(&owner.relative, contract::CONTRACT_MESSAGE));
    }
}

fn inspect_file(file: &SourceFile, seen: &mut Seen, violations: &mut Vec<Violation>) {
    let source = RustSource::parse(&file.text);
    let verifier_visible = source
        .tokens
        .iter()
        .any(|token| token.identifier(contract::HOST_VERIFIER));
    for (index, token) in source.tokens.iter().enumerate() {
        if test_guarded(&source, index) {
            continue;
        }
        if sites::method_symbol(token) {
            inspect_method(file, &source, index, verifier_visible, seen, violations);
        }
        if sites::constructor_symbol(token) && syntax::constructor(&source.tokens, index) {
            inspect_constructor(file, &source, index, seen, violations);
        }
        if token.identifier(contract::SELF_TYPE) && syntax::constructor(&source.tokens, index) {
            inspect_self_constructor(file, &source, index, violations);
        }
    }
}

fn inspect_method(
    file: &SourceFile,
    source: &RustSource,
    index: usize,
    verifier_visible: bool,
    seen: &mut Seen,
    violations: &mut Vec<Violation>,
) {
    let symbol = sites::identifier(&source.tokens[index]).unwrap_or_default();
    let occurrence = syntax::method_occurrence(&source.tokens, index);
    if symbol == contract::ACKNOWLEDGE
        && !sites::acknowledgement_relevant(source, index, occurrence, verifier_visible)
    {
        return;
    }
    let Some(occurrence) = occurrence else {
        reject(file, &source.tokens[index], violations);
        return;
    };
    let function = source.function_at(index);
    let expected = contract::METHOD_SITES.iter().position(|site| {
        sites::matches(site, file, function, symbol, occurrence)
            && syntax::route_matches(site.route, &source.tokens, index)
    });
    record(
        expected,
        &mut seen.methods,
        file,
        &source.tokens[index],
        violations,
    );
}

fn inspect_constructor(
    file: &SourceFile,
    source: &RustSource,
    index: usize,
    seen: &mut Seen,
    violations: &mut Vec<Violation>,
) {
    let symbol = sites::identifier(&source.tokens[index]).unwrap_or_default();
    let expected = contract::CONSTRUCTOR_SITES.iter().position(|site| {
        sites::matches(
            site,
            file,
            source.function_at(index),
            symbol,
            contract::Occurrence::Constructor,
        )
    });
    record(
        expected,
        &mut seen.constructors,
        file,
        &source.tokens[index],
        violations,
    );
}

fn inspect_self_constructor(
    file: &SourceFile,
    source: &RustSource,
    index: usize,
    violations: &mut Vec<Violation>,
) {
    if source
        .function_at(index)
        .and_then(|function| function.owner.as_deref())
        .is_some_and(sites::protected_constructor_owner)
    {
        reject(file, &source.tokens[index], violations);
    }
}

fn record(
    expected: Option<usize>,
    seen: &mut [usize],
    file: &SourceFile,
    token: &Token,
    violations: &mut Vec<Violation>,
) {
    let Some(position) = expected else {
        reject(file, token, violations);
        return;
    };
    seen[position] += contract::EXPECTED_SITE_OCCURRENCES;
    if seen[position] != contract::EXPECTED_SITE_OCCURRENCES {
        reject(file, token, violations);
    }
}

fn incomplete(seen: &Seen) -> bool {
    seen.methods
        .iter()
        .chain(&seen.constructors)
        .any(|count| *count != contract::EXPECTED_SITE_OCCURRENCES)
}

fn test_guarded(source: &RustSource, index: usize) -> bool {
    source
        .function_at(index)
        .is_some_and(|function| source.is_test_guarded(function))
}

fn production_rust(file: &SourceFile) -> bool {
    yolo_architecture_source_policy::source_policy::is_rust_product_source(&file.relative)
}

fn zeroed(sites: &[contract::Site]) -> Vec<usize> {
    vec![usize::default(); sites.len()]
}

fn source<'a>(files: &'a [SourceFile], path: &str) -> Option<&'a SourceFile> {
    files.iter().find(|file| file.relative == path)
}

fn reject(file: &SourceFile, token: &Token, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(
        &file.relative,
        token.line,
        contract::POLICY_MESSAGE,
    ));
}
