use crate::constants::{
    ALIAS_RULE, FILEGROUP_RULE, ISOLATED_RUST_TEST_RULE, LOAD_FUNCTION, RUST_BINARY_RULE,
    RUST_LIBRARY_RULE, RUST_STATIC_LIBRARY_RULE, RUST_TEST_RULE, STARLARK_CONTROL_KEYWORDS,
};
use crate::expression_parser::ExpressionParser;
use crate::model::{Assignment, CallExpression, CallSite, Document, Expression, RuleCall};
use crate::paths;
use std::collections::BTreeMap;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_build_graph::crates::RuleOrigins;
use yolo_architecture_syntax::cargo_starlark_tokens::{Token, tokenize};

const NEXT_TOKEN: usize = 1;
const ASSIGNMENT_WIDTH: usize = 2;

pub fn document(file: &SourceFile) -> Document {
    let tokens = tokenize(&file.text);
    let origins = RuleOrigins::parse(&tokens);
    let mut assignments: BTreeMap<String, Vec<Assignment>> = BTreeMap::new();
    let mut rules = Vec::new();
    let mut imports = BTreeMap::new();
    let mut calls = Vec::new();
    let mut dynamic_call = false;
    let mut depth = Depth::default();
    for cursor in 0..tokens.len() {
        let token = &tokens[cursor];
        if token.symbol('(') && cursor > 0 {
            dynamic_call |=
                tokens[cursor - NEXT_TOKEN].symbol(')') || tokens[cursor - NEXT_TOKEN].symbol(']');
        }
        if let Some(name) = token
            .identifier()
            .filter(|name| !STARLARK_CONTROL_KEYWORDS.contains(name))
            && tokens
                .get(cursor + NEXT_TOKEN)
                .is_some_and(|next| next.symbol('('))
        {
            calls.push(CallSite {
                name: name.to_owned(),
                canonical: origins.canonical(name).to_owned(),
                line: token.line,
                top_level: depth.top_level(),
                qualified: cursor > 0 && tokens[cursor - NEXT_TOKEN].symbol('.'),
                definition: cursor > 0 && tokens[cursor - NEXT_TOKEN].identifier() == Some("def"),
                depset_receiver:
                    yolo_architecture_build_graph::crates::direct_call_receiver(
                        &tokens, cursor, "depset",
                    ),
            });
        }
        if depth.top_level() {
            parse_assignment(&tokens, cursor, &mut assignments);
        }
        if depth.top_level()
            && let Some(name) = token.identifier().map(|name| {
                if schema_macro(name, &imports) {
                    RUST_LIBRARY_RULE
                } else {
                    origins.canonical(name)
                }
            })
            && matches!(
                name,
                RUST_LIBRARY_RULE
                    | RUST_STATIC_LIBRARY_RULE
                    | RUST_BINARY_RULE
                    | RUST_TEST_RULE
                    | ISOLATED_RUST_TEST_RULE
                    | FILEGROUP_RULE
                    | ALIAS_RULE
                    | LOAD_FUNCTION
            )
            && tokens
                .get(cursor + NEXT_TOKEN)
                .is_some_and(|next| next.symbol('('))
            && let Some(Expression::Call(call)) = ExpressionParser::new(&tokens, cursor).parse()
        {
            if name == LOAD_FUNCTION {
                record_imports(&file.relative, &call, &mut imports);
            } else {
                let mut attributes = call.attributes;
                if token
                    .identifier()
                    .is_some_and(|name| schema_macro(name, &imports))
                {
                    let source = attributes
                        .get(crate::constants::CRATE_ROOT_ATTRIBUTE)
                        .cloned()
                        .unwrap_or(Expression::List(Vec::new()));
                    attributes.insert(crate::constants::SOURCES_ATTRIBUTE.into(), source);
                }
                rules.push(RuleCall {
                    name: name.to_owned(),
                    line: token.line,
                    attributes,
                });
            }
        }
        depth.observe(token);
    }
    Document {
        path: file.relative.clone(),
        package: paths::package(&file.relative),
        assignments,
        imports,
        rules,
        rules_unresolved: origins.uncertain() || dynamic_call,
        calls,
    }
}

fn schema_macro(name: &str, imports: &BTreeMap<String, String>) -> bool {
    imports.get(name).is_some_and(|origin| {
        let (path, symbol) = origin.split_once('#').unwrap_or((origin, name));
        path == yolo_architecture_policy_contracts::entrypoint_macros::CAPNP_DEFINITIONS
            && symbol == yolo_architecture_policy_contracts::entrypoint_macros::CAPNP_LIBRARY_MACRO
    })
}

fn parse_assignment(
    tokens: &[Token],
    cursor: usize,
    assignments: &mut BTreeMap<String, Vec<Assignment>>,
) {
    let Some(name) = tokens[cursor].identifier() else {
        return;
    };
    if !tokens
        .get(cursor + NEXT_TOKEN)
        .is_some_and(|token| token.symbol('='))
    {
        return;
    }
    let Some(expression) = ExpressionParser::new(tokens, cursor + ASSIGNMENT_WIDTH).parse() else {
        return;
    };
    assignments
        .entry(name.to_owned())
        .or_default()
        .push(Assignment {
            line: tokens[cursor].line,
            expression,
        });
}

fn record_imports(path: &str, call: &CallExpression, imports: &mut BTreeMap<String, String>) {
    let Some(Expression::String(label)) = call.positional.first() else {
        return;
    };
    let current_package = paths::package(path);
    let Some((package, name)) = paths::label_parts(&current_package, label) else {
        return;
    };
    let source = if package.is_empty() {
        name
    } else {
        format!("{package}/{name}")
    };
    for expression in call.positional.iter().skip(NEXT_TOKEN) {
        if let Expression::String(symbol) = expression {
            imports.insert(symbol.clone(), source.clone());
        }
    }
    for (alias, expression) in &call.attributes {
        if let Expression::String(symbol) = expression {
            imports.insert(alias.clone(), format!("{source}#{symbol}"));
        }
    }
}

#[derive(Default)]
struct Depth {
    parens: usize,
    brackets: usize,
    braces: usize,
}

impl Depth {
    fn top_level(&self) -> bool {
        self.parens == 0 && self.brackets == 0 && self.braces == 0
    }

    fn observe(&mut self, token: &Token) {
        for (open, close, depth) in [
            ('(', ')', &mut self.parens),
            ('[', ']', &mut self.brackets),
            ('{', '}', &mut self.braces),
        ] {
            if token.symbol(open) {
                *depth += 1;
            } else if token.symbol(close) {
                *depth = depth.saturating_sub(1);
            }
        }
    }
}
