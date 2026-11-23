use crate::pty_master_closure_contract as contract;
use yolo_architecture_syntax::targeted_tokens::Token;

const CLONED_RECEIVER_PREFIX_LENGTH: usize = 6;
const NEXT_TOKEN_OFFSET: usize = 1;
const ARGUMENT_TOKEN_LENGTH: usize = 3;
const CLONE_METHOD: &str = "clone";
const MEMBER_ACCESS: &str = ".";
const LEFT_PAREN: &str = "(";
const RIGHT_PAREN: &str = ")";

pub fn cloned_verifier_acknowledgement(tokens: &[Token], index: usize, argument: &str) -> bool {
    let Some(prefix_start) = index.checked_sub(CLONED_RECEIVER_PREFIX_LENGTH) else {
        return false;
    };
    let Some(argument_start) = index.checked_add(NEXT_TOKEN_OFFSET) else {
        return false;
    };
    let Some(argument_end) = argument_start.checked_add(ARGUMENT_TOKEN_LENGTH) else {
        return false;
    };
    let Some([receiver, access, clone, left, right, second_access]) =
        tokens.get(prefix_start..index)
    else {
        return false;
    };
    let Some([call_left, exact, call_right]) = tokens.get(argument_start..argument_end) else {
        return false;
    };
    receiver.identifier(contract::VERIFIER_FIELD)
        && access.symbol(MEMBER_ACCESS)
        && clone.identifier(CLONE_METHOD)
        && left.symbol(LEFT_PAREN)
        && right.symbol(RIGHT_PAREN)
        && second_access.symbol(MEMBER_ACCESS)
        && call_left.symbol(LEFT_PAREN)
        && exact.identifier(argument)
        && call_right.symbol(RIGHT_PAREN)
}
