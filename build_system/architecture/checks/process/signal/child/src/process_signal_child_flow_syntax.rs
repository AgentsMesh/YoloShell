use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_token_sequence as sequence;
use yolo_architecture_syntax::rust_source::RustFunction;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const CALL_OPEN: &str = "(";
const CALL_CLOSE: &str = ")";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const MEMBER_ACCESS: &str = ".";
const ASSIGNMENT: &str = "=";
const ARROW_END: &str = ">";
const REFERENCE: &str = "&";
const RETURN_KEYWORD: &str = "return";
const IF_KEYWORD: &str = "if";
const LET_KEYWORD: &str = "let";
const MATCH_KEYWORD: &str = "match";
const MUTABLE_KEYWORD: &str = "mut";
const SOME_VARIANT: &str = "Some";
const NONE_VARIANT: &str = "None";
const OK_VARIANT: &str = "Ok";
const ERROR_VARIANT: &str = "Err";
const ERROR_BINDING: &str = "error";
const TERMINATED_BINDING: &str = "terminated";
const OBSERVED_BINDING: &str = "observed";
const TAKE_METHOD: &str = "take";
const LOSSY_OK_METHOD: &str = "ok";
const ARM_OPEN_OFFSET: usize = 6;
const ROOTED_CHILD_PREFIX_LENGTH: usize = 7;

pub fn exact_child_extraction(tokens: &[Token], function: &RustFunction) -> bool {
    function.body.clone().any(|index| {
        sequence::preceding(
            tokens,
            index,
            &[LET_KEYWORD, SOME_VARIANT, CALL_OPEN, MUTABLE_KEYWORD],
        ) && tokens[index].identifier(contract::CHILD_RECEIVER)
            && sequence::following(
                tokens,
                index,
                &[
                    CALL_CLOSE,
                    ASSIGNMENT,
                    contract::SELF_RECEIVER,
                    MEMBER_ACCESS,
                    contract::CHILD_RECEIVER,
                    MEMBER_ACCESS,
                    TAKE_METHOD,
                    CALL_OPEN,
                ],
            )
    })
}

pub fn child_binding_count(tokens: &[Token], function: &RustFunction) -> usize {
    function
        .body
        .clone()
        .filter(|index| {
            tokens[*index].identifier(contract::CHILD_RECEIVER)
                && index
                    .checked_sub(1)
                    .is_some_and(|previous| tokens[previous].identifier(MUTABLE_KEYWORD))
                && index
                    .checked_sub(2)
                    .is_none_or(|previous| !tokens[previous].symbol(REFERENCE))
        })
        .count()
}

pub fn match_arm_contains(
    tokens: &[Token],
    range: std::ops::Range<usize>,
    kill: usize,
    wait: usize,
) -> bool {
    range.into_iter().any(|index| {
        tokens[index].identifier(OK_VARIANT)
            && sequence::following(
                tokens,
                index,
                &[
                    CALL_OPEN,
                    NONE_VARIANT,
                    CALL_CLOSE,
                    ASSIGNMENT,
                    ARROW_END,
                    BLOCK_OPEN,
                ],
            )
            && closing(tokens, index + ARM_OPEN_OFFSET, BLOCK_OPEN, BLOCK_CLOSE)
                .is_some_and(|close| index < kill && wait < close)
    })
}

pub fn assignment_before(tokens: &[Token], call: usize) -> bool {
    call.checked_sub(ROOTED_CHILD_PREFIX_LENGTH)
        .is_some_and(|root| {
            sequence::preceding(tokens, root, &[LET_KEYWORD, TERMINATED_BINDING, ASSIGNMENT])
        })
}

pub fn termination_error_returns(tokens: &[Token], start: usize, end: usize) -> bool {
    (start..end).any(|index| {
        sequence::preceding(tokens, index, &[IF_KEYWORD, LET_KEYWORD])
            && tokens[index].identifier(ERROR_VARIANT)
            && sequence::following(
                tokens,
                index,
                &[
                    CALL_OPEN,
                    ERROR_BINDING,
                    CALL_CLOSE,
                    ASSIGNMENT,
                    TERMINATED_BINDING,
                    BLOCK_OPEN,
                ],
            )
            && closing(tokens, index + ARM_OPEN_OFFSET, BLOCK_OPEN, BLOCK_CLOSE).is_some_and(
                |close| {
                    close < end
                        && sequence::contains_top_level_identifier(
                            tokens,
                            index + ARM_OPEN_OFFSET,
                            close,
                            RETURN_KEYWORD,
                        )
                        && sequence::contains_top_level_sequence(
                            tokens,
                            index + ARM_OPEN_OFFSET,
                            close,
                            &[
                                contract::SETTLE_FAILED_CHILD,
                                CALL_OPEN,
                                contract::CHILD_RECEIVER,
                            ],
                        )
                },
            )
    })
}

pub fn poll_error_settles(tokens: &[Token], range: std::ops::Range<usize>) -> bool {
    range.into_iter().any(|index| {
        tokens[index].identifier(ERROR_VARIANT)
            && sequence::following(
                tokens,
                index,
                &[
                    CALL_OPEN,
                    ERROR_BINDING,
                    CALL_CLOSE,
                    ASSIGNMENT,
                    ARROW_END,
                    contract::SETTLE_FAILED_CHILD,
                    CALL_OPEN,
                    contract::CHILD_RECEIVER,
                ],
            )
    })
}

pub fn settlement_match(
    tokens: &[Token],
    function: &RustFunction,
) -> Option<std::ops::Range<usize>> {
    let mut ranges = function.body.clone().filter_map(|index| {
        (tokens[index].identifier(MATCH_KEYWORD)
            && sequence::following(tokens, index, &[OBSERVED_BINDING, BLOCK_OPEN]))
        .then(|| index + 2)
        .and_then(|open| closing(tokens, open, BLOCK_OPEN, BLOCK_CLOSE).map(|close| open..close))
    });
    let range = ranges.next()?;
    ranges.next().is_none().then_some(range)
}

pub fn lossy_poll_conversion(function: &RustFunction, tokens: &[Token]) -> bool {
    function.body.clone().any(|index| {
        tokens[index].identifier(LOSSY_OK_METHOD)
            && index > 0
            && tokens[index - 1].symbol(MEMBER_ACCESS)
    })
}
