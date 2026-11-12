use crate::process_signal_retained_receiver;
use std::collections::BTreeSet;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_path_analysis::types::{
    RetainedFields, RetainedTypes, type_segment_contains,
};
use yolo_architecture_syntax::rust_source::{RustFunction, identifier};
use yolo_architecture_syntax::targeted_tokens::Token;

const MEMBER_ACCESS: &str = ".";
const TYPE_SEPARATOR: &str = ":";
const PATH_SEPARATOR: &str = "::";
const GROUP_OPEN: &str = "(";
const GROUP_CLOSE: &str = ")";
const ARRAY_OPEN: &str = "[";
const ARRAY_CLOSE: &str = "]";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const WILDCARD: &str = "_";
const MUT_KEYWORD: &str = "mut";
const REF_KEYWORD: &str = "ref";

#[path = "process_signal_retained_dataflow/patterns.rs"]
mod patterns;

pub fn bindings(
    tokens: &[Token],
    function: &RustFunction,
    retained: &RetainedTypes,
    fields: &RetainedFields,
) -> BTreeSet<String> {
    let mut bindings = typed_bindings(tokens, function, retained);
    propagate(tokens, function, retained, fields, &mut bindings);
    bindings
}

fn typed_bindings(
    tokens: &[Token],
    function: &RustFunction,
    retained: &RetainedTypes,
) -> BTreeSet<String> {
    let mut bindings = BTreeSet::new();
    for index in function.body.start..function.body.end {
        if !tokens[index].symbol(TYPE_SEPARATOR) || index == 0 {
            continue;
        }
        if !type_segment_contains(tokens, index + 1, function.body.end, retained) {
            continue;
        }
        let start = process_signal_retained_receiver::typed_binding_start(
            tokens,
            function.body.start,
            index,
        );
        bindings.extend(binding_names(tokens, start, index));
    }
    bindings
}

fn propagate(
    tokens: &[Token],
    function: &RustFunction,
    retained: &RetainedTypes,
    fields: &RetainedFields,
    bindings: &mut BTreeSet<String>,
) {
    let flows = patterns::flows(tokens, function.body.clone());
    loop {
        let mut changed = false;
        for flow in &flows {
            let inherited = expression_retained(
                tokens,
                flow.source.start,
                flow.source.end,
                function,
                retained,
                fields,
                bindings,
            );
            let retained_pattern = flow
                .pattern
                .clone()
                .any(|index| retained.contains(tokens, index));
            if inherited || retained_pattern {
                for name in binding_names(tokens, flow.pattern.start, flow.pattern.end) {
                    changed |= bindings.insert(name);
                }
            }
        }
        if !changed {
            return;
        }
    }
}

fn expression_retained(
    tokens: &[Token],
    start: usize,
    end: usize,
    function: &RustFunction,
    retained: &RetainedTypes,
    fields: &RetainedFields,
    bindings: &BTreeSet<String>,
) -> bool {
    (start..end).any(|index| {
        retained.contains(tokens, index)
            || identifier(&tokens[index]).is_some_and(|name| bindings.contains(name))
    }) || self_field_reference(&tokens[start..end], function, fields)
}

fn self_field_reference(
    tokens: &[Token],
    function: &RustFunction,
    fields: &RetainedFields,
) -> bool {
    let Some(owner) = function.owner.as_deref() else {
        return false;
    };
    tokens.windows(3).any(|window| {
        window[0].identifier(contract::SELF_RECEIVER)
            && window[1].symbol(MEMBER_ACCESS)
            && identifier(&window[2])
                .is_some_and(|field| fields.contains(&(owner.to_owned(), field.to_owned())))
    })
}

fn binding_names(tokens: &[Token], start: usize, end: usize) -> Vec<String> {
    let mut braces = 0_usize;
    let mut names = Vec::new();
    for index in start..end {
        if tokens[index].symbol(BLOCK_OPEN) {
            braces += 1;
            continue;
        }
        if tokens[index].symbol(BLOCK_CLOSE) {
            braces = braces.saturating_sub(1);
            continue;
        }
        let Some(name) = identifier(&tokens[index]) else {
            continue;
        };
        let previous_path = index > start && tokens[index - 1].symbol(PATH_SEPARATOR);
        let next_path = tokens
            .get(index + 1)
            .is_some_and(|token| token.symbol(PATH_SEPARATOR));
        let constructor = tokens
            .get(index + 1)
            .is_some_and(|token| token.symbol(GROUP_OPEN) || token.symbol(BLOCK_OPEN));
        let field = braces > 0
            && tokens
                .get(index + 1)
                .is_some_and(|token| token.symbol(TYPE_SEPARATOR));
        let modifier = matches!(name, MUT_KEYWORD | REF_KEYWORD)
            && tokens.get(index + 1).and_then(identifier).is_some();
        if name != WILDCARD && !previous_path && !next_path && !constructor && !field && !modifier {
            names.push(name.to_owned());
        }
    }
    names
}

#[derive(Default)]
struct Depth {
    groups: usize,
    arrays: usize,
    blocks: usize,
}

impl Depth {
    fn is_top(&self) -> bool {
        self.groups == 0 && self.arrays == 0 && self.blocks == 0
    }

    fn observe(&mut self, token: &Token) {
        if token.symbol(GROUP_OPEN) {
            self.groups += 1;
        } else if token.symbol(GROUP_CLOSE) {
            self.groups = self.groups.saturating_sub(1);
        } else if token.symbol(ARRAY_OPEN) {
            self.arrays += 1;
        } else if token.symbol(ARRAY_CLOSE) {
            self.arrays = self.arrays.saturating_sub(1);
        } else if token.symbol(BLOCK_OPEN) {
            self.blocks += 1;
        } else if token.symbol(BLOCK_CLOSE) {
            self.blocks = self.blocks.saturating_sub(1);
        }
    }
}
