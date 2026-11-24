use crate::pty_master_closure_contract::{self as contract, Occurrence, Route};
use crate::pty_master_closure_syntax as syntax;
use yolo_architecture_syntax::targeted_tokens::{Kind, Token, tokenize};

#[path = "../test_support/constants.rs"]
mod constants;
use constants::MARKER;
const OTHER: &str = "unrelated";
const OVERFLOW_DISTANCE: usize = 1;

fn indexed(source: &str, symbol: &str) -> (Vec<Token>, usize) {
    let tokens = tokenize(source);
    let index = tokens
        .iter()
        .position(|token| token.identifier(symbol))
        .unwrap();
    (tokens, index)
}

#[test]
fn method_and_constructor_classification_handles_boundaries_and_item_headers() {
    for (source, expected) in [
        (
            format!("pub fn {MARKER}() {{}}"),
            Some(Occurrence::Definition),
        ),
        (format!("value.{MARKER}()"), Some(Occurrence::Call)),
        (format!("value.{MARKER}"), None),
        (MARKER.to_owned(), None),
    ] {
        let (tokens, index) = indexed(&source, MARKER);
        assert!(syntax::method_occurrence(&tokens, index) == expected);
    }
    for index in [usize::default(), usize::MAX] {
        assert!(syntax::method_occurrence(&[], index).is_none());
        assert!(!syntax::constructor(&[], index));
        assert!(!syntax::crate_definition(&[], index));
        assert!(!syntax::qualified_verifier_acknowledgement(&[], index));
    }
    for header in ["fn function() ->", "struct", "impl", "enum"] {
        let (tokens, index) = indexed(&format!("{header} {MARKER} {{}}"), MARKER);
        assert!(!syntax::constructor(&tokens, index), "{header}");
    }
    for source in [
        format!("{MARKER} {{}}"),
        format!("fn function() {{ let value = {MARKER} {{}}; }}"),
        format!("struct Previous {{}} let value = {MARKER} {{}};"),
    ] {
        let (tokens, index) = indexed(&source, MARKER);
        assert!(syntax::constructor(&tokens, index), "{source}");
    }
    for (prefix, expected) in [
        ("pub fn", true),
        ("fn", false),
        ("pub let", false),
        ("let fn", false),
    ] {
        let (tokens, index) = indexed(&format!("{prefix} {MARKER}() {{}}"), MARKER);
        assert_eq!(
            syntax::route_matches(Route::Definition, &tokens, index),
            expected
        );
    }
}

#[test]
fn exact_and_cloned_acknowledgements_reject_every_changed_operand() {
    let plain = format!(
        "{}.{}.{MARKER}({})",
        contract::SELF_RECEIVER,
        contract::VERIFIER_FIELD,
        contract::EXACT_ACK_ARGUMENT
    );
    let startup = format!(
        "{}.clone().{MARKER}({})",
        contract::VERIFIER_FIELD,
        contract::EXACT_ACK_ARGUMENT
    );
    let gone = format!(
        "{}.clone().{MARKER}({})",
        contract::VERIFIER_FIELD,
        contract::HOLDER_GONE_ARGUMENT
    );
    for (route, source) in [
        (Route::HostAcknowledgement, plain),
        (Route::StartupHostAcknowledgement, startup),
        (Route::HolderGoneAcknowledgement, gone),
    ] {
        let (tokens, index) = indexed(&source, MARKER);
        assert!(syntax::route_matches(route, &tokens, index));
        for changed in (0..tokens.len()).filter(|changed| *changed != index) {
            let mut candidate = tokens.clone();
            candidate[changed].kind = Kind::Number(u64::default());
            assert!(
                !syntax::route_matches(route, &candidate, index),
                "{source}: {changed}"
            );
        }
        for end in index..tokens.len() {
            assert!(!syntax::route_matches(route, &tokens[..end], index));
        }
        for index in [usize::default(), usize::MAX - OVERFLOW_DISTANCE, usize::MAX] {
            assert!(!syntax::route_matches(route, &[], index));
        }
    }
}

#[test]
fn qualified_acknowledgement_requires_its_exact_type_and_path_separator() {
    for (owner, separator, expected) in [
        (contract::HOST_VERIFIER, "::", true),
        (OTHER, "::", false),
        (contract::HOST_VERIFIER, ".", false),
    ] {
        let (tokens, index) = indexed(&format!("{owner}{separator}{MARKER}()"), MARKER);
        assert_eq!(
            syntax::qualified_verifier_acknowledgement(&tokens, index),
            expected
        );
    }
}

#[test]
fn each_settlement_route_rejects_missing_duplicate_and_foreign_proof_steps() {
    for (route, required, forbidden) in [
        (
            Route::LocalClose,
            vec![contract::TAKE_LOCAL],
            vec![
                contract::FINALIZE_HOST,
                contract::FINALIZE_LOCAL,
                contract::MINT_FINAL,
            ],
        ),
        (
            Route::HostSettlement,
            vec![contract::FINALIZE_HOST, contract::MINT_FINAL],
            vec![
                contract::FINALIZE_LOCAL,
                contract::TAKE_LOCAL,
                contract::ACKNOWLEDGE,
            ],
        ),
        (
            Route::NoSecondaryCustody,
            vec![
                contract::TAKE_LOCAL,
                contract::FINALIZE_LOCAL,
                contract::MINT_FINAL,
            ],
            vec![contract::FINALIZE_HOST, contract::ACKNOWLEDGE],
        ),
    ] {
        let accepts = |steps: &[&str]| {
            let source = format!(
                "{MARKER}({})",
                steps
                    .iter()
                    .map(|step| format!("{step}()"))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let (tokens, index) = indexed(&source, MARKER);
            syntax::route_matches(route, &tokens, index)
        };
        assert!(accepts(&required));
        for missing in 0..required.len() {
            let mut steps = required.clone();
            steps.remove(missing);
            assert!(!accepts(&steps));
            let mut steps = required.clone();
            steps.push(required[missing]);
            assert!(!accepts(&steps));
        }
        for forbidden in forbidden {
            let mut steps = required.clone();
            steps.push(forbidden);
            assert!(!accepts(&steps));
            let source = format!(
                "{forbidden}(); {MARKER}({}); {forbidden}();",
                required.join(",")
            );
            let (tokens, index) = indexed(&source, MARKER);
            assert!(syntax::route_matches(route, &tokens, index));
        }
    }
}
#[test]
fn cloned_acknowledgement_rejects_both_index_addition_overflows() {
    for index in [usize::MAX, usize::MAX - 1] {
        assert!(
            !crate::pty_master_closure_ack_syntax::cloned_verifier_acknowledgement(
                &[],
                index,
                "argument"
            )
        );
    }
}
