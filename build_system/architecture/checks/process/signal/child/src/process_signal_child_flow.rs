use crate::process_signal_child_flow_bridge_bindings as bridge_bindings;
use crate::process_signal_child_flow_bridge_dataflow as bridge_dataflow;
use crate::process_signal_child_flow_remote_reaper as remote_reaper;
use crate::process_signal_child_flow_remote_settlement as remote_settlement;
use crate::process_signal_child_flow_syntax as syntax;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};
use yolo_architecture_syntax::rust_source_cfg;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const CALL_OPEN: &str = "(";
const NEXT_TOKEN_OFFSET: usize = 1;
const REQUIRED_CALL_COUNT: usize = 1;
const REQUIRED_CHILD_BINDING_COUNT: usize = 1;
const ATTRIBUTE_MARKER: &str = "#";
const ATTRIBUTE_OPEN: &str = "[";
const ATTRIBUTE_CLOSE: &str = "]";
const CFG_ATTRIBUTE: &str = "cfg";
const ASSIGNMENT: &str = "=";

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    let valid = match file.relative.as_str() {
        contract::BRIDGE_CHILD_PATH => bridge_flow(source),
        contract::BROKER_PENDING_PATH => remote_settlement::spawned_flow(source),
        contract::BROKER_SETTLEMENT_PATH => remote_settlement::settlement_flow(source),
        contract::BROKER_REAPER_CUSTODY_PATH => remote_reaper::custody_flow(source),
        contract::BROKER_REAPER_WORKER_PATH => remote_reaper::worker_flow(source),
        _ => return,
    };
    if !valid {
        violations.push(Violation::file(
            &file.relative,
            contract::CHILD_FLOW_MESSAGE,
        ));
    }
}

fn bridge_flow(source: &RustSource) -> bool {
    let Some(function) = unique_function(source, Some(contract::SPAWNED_HOST), contract::DROP)
    else {
        return false;
    };
    if production_cfg_in_body(&source.tokens, function) {
        return false;
    }
    let tokens = &source.tokens;
    let kills = calls(tokens, function, contract::KILL_METHOD);
    let waits = calls(tokens, function, contract::WAIT_METHOD);
    let polls = calls(tokens, function, contract::TRY_WAIT);
    let Some((&kill, &wait)) = kills.first().zip(waits.first()) else {
        return false;
    };
    let Some(match_range) = syntax::settlement_match(tokens, function) else {
        return false;
    };
    kills.len() == REQUIRED_CALL_COUNT
        && waits.len() == REQUIRED_CALL_COUNT
        && polls.len() == REQUIRED_CALL_COUNT
        && kill < wait
        && syntax::exact_child_extraction(tokens, function)
        && syntax::child_binding_count(tokens, function) == REQUIRED_CHILD_BINDING_COUNT
        && bridge_bindings::stable_child_binding(tokens, function)
        && bridge_dataflow::exact_poll_flow(tokens, function, &polls)
        && bridge_dataflow::exact_rooted_child_call(tokens, kill, contract::KILL_METHOD)
        && bridge_dataflow::exact_termination_result(tokens, function, kill)
        && bridge_dataflow::exact_rooted_child_call(tokens, wait, contract::WAIT_METHOD)
        && bridge_dataflow::exact_wait_guard(tokens, wait)
        && bridge_dataflow::calls_are_top_level(tokens, match_range.clone(), kill, wait)
        && bridge_bindings::no_settlement_shadowing(tokens, function)
        && syntax::match_arm_contains(tokens, match_range.clone(), kill, wait)
        && syntax::assignment_before(tokens, kill)
        && syntax::termination_error_returns(tokens, kill, wait)
        && syntax::poll_error_settles(tokens, match_range)
        && !syntax::lossy_poll_conversion(function, tokens)
}

pub(super) fn unique_function<'a>(
    source: &'a RustSource,
    owner: Option<&str>,
    name: &str,
) -> Option<&'a RustFunction> {
    let mut functions = source
        .functions()
        .filter(|function| function.matches(owner, name));
    let function = functions.next()?;
    functions.next().is_none().then_some(function)
}

pub(super) fn production_cfg_in_body(tokens: &[Token], function: &RustFunction) -> bool {
    let mut cursor = function.body.start;
    while cursor + 2 < function.body.end {
        if !tokens[cursor].symbol(ATTRIBUTE_MARKER)
            || !tokens[cursor + 1].symbol(ATTRIBUTE_OPEN)
            || !tokens[cursor + 2].identifier(CFG_ATTRIBUTE)
        {
            cursor += 1;
            continue;
        }
        let Some(close) = closing(tokens, cursor + 1, ATTRIBUTE_OPEN, ATTRIBUTE_CLOSE) else {
            return true;
        };
        if !rust_source_cfg::is_unit_test_guarded(&tokens[cursor..=close]) {
            return true;
        }
        cursor = close + 1;
    }
    false
}

pub(super) fn calls(tokens: &[Token], function: &RustFunction, name: &str) -> Vec<usize> {
    function
        .body
        .clone()
        .filter(|index| {
            tokens[*index].identifier(name)
                && tokens
                    .get(index + NEXT_TOKEN_OFFSET)
                    .is_some_and(|token| token.symbol(CALL_OPEN))
                && !yolo_architecture_process_signal_foundation::process_signal_token_sequence::macro_argument(tokens, *index)
        })
        .collect()
}

pub(super) fn assignment_count(
    source: &RustSource,
    function: &RustFunction,
    binding: &str,
    value: &str,
) -> usize {
    sequence_positions(source, function, &[binding, ASSIGNMENT, value]).len()
}

pub(super) fn ordered_once(
    source: &RustSource,
    function: &RustFunction,
    first: &str,
    second: &str,
) -> bool {
    matches!((one_call(source, function, first), one_call(source, function, second)),
        (Some(first), Some(second)) if first < second)
}

pub(super) fn one_call(source: &RustSource, function: &RustFunction, name: &str) -> Option<usize> {
    let calls = calls(&source.tokens, function, name);
    (calls.len() == REQUIRED_CALL_COUNT).then(|| calls[0])
}

pub(super) fn contains(source: &RustSource, function: &RustFunction, expected: &[&str]) -> bool {
    !sequence_positions(source, function, expected).is_empty()
}

pub(super) fn sequence_before(
    source: &RustSource,
    function: &RustFunction,
    first: &[&str],
    second: &[&str],
) -> bool {
    matches!((sequence_positions(source, function, first).as_slice(), sequence_positions(source, function, second).as_slice()),
        ([first], [second]) if first < second)
}

fn sequence_positions(
    source: &RustSource,
    function: &RustFunction,
    expected: &[&str],
) -> Vec<usize> {
    source.tokens[function.body.clone()]
        .windows(expected.len())
        .enumerate()
        .filter(|(_, window)| {
            window
                .iter()
                .zip(expected)
                .all(|(token, value)| token.identifier(value) || token.symbol(value))
        })
        .map(|(offset, _)| function.body.start + offset)
        .collect()
}
