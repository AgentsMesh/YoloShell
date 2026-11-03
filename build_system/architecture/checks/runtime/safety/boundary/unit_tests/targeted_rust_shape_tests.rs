use crate::targeted_rust_rules::*;
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::targeted_tokens::{Token, tokenize};

const SOURCE_PATH: &str = "core/crates/runtime/engine/host/src/security.rs";
type Check = fn(&SourceFile, &[Token], &mut Vec<Violation>);

fn count(source: &str, check: Check) -> usize {
    let file = SourceFile {
        absolute: SOURCE_PATH.into(),
        relative: SOURCE_PATH.into(),
        text: source.into(),
    };
    let mut violations = Vec::new();
    check(&file, &tokenize(source), &mut violations);
    violations.len()
}

#[test]
fn permission_masks_require_the_exact_call_shape_and_a_numeric_expression() {
    for suffix in [" == 0", " != 0", ",", ";", "{", "}", ""] {
        let source = format!("metadata.mode() & 63{suffix}");
        assert_eq!(count(&source, inspect_permission_masks), 1, "{source}");
    }
    for source in [
        "mode",
        "mode(",
        "mode()",
        "mode() &",
        "mode[] & 63",
        "mode(63) & 63",
        "mode() | 63",
        "mode() & NAMED_MASK",
        "flags & 63",
    ] {
        assert_eq!(count(source, inspect_permission_masks), 0, "{source}");
    }
}

#[test]
fn hashed_positions_distinguish_numeric_methods_from_incomplete_or_named_calls() {
    for method in ["nth", "skip"] {
        for argument in ["1", "(1 + 1)", "0x2"] {
            let source = format!("parts.{method}({argument})");
            assert_eq!(count(&source, inspect_hashed_host_fields), 1, "{source}");
        }
        for source in [
            method.to_owned(),
            format!("{method}(1)"),
            format!("parts::{method}(1)"),
            format!("parts.{method}"),
            format!("parts.{method}[1]"),
            format!("parts.{method}(1"),
            format!("parts.{method}(NAMED_INDEX)"),
            format!("parts.{method}()"),
        ] {
            assert_eq!(count(&source, inspect_hashed_host_fields), 0, "{source}");
        }
    }
}

#[test]
fn hex_capacity_scaling_checks_both_operands_and_rejects_truncated_shapes() {
    for argument in [
        "digest.len() * 2",
        "2 * hash.len()",
        "output_size << 1",
        "hash.len() * (1 + 1)",
    ] {
        let source = format!("String::with_capacity({argument})");
        assert_eq!(count(&source, inspect_hex_capacities), 1, "{source}");
    }
    for source in [
        "with_capacity",
        "with_capacity[digest * 2]",
        "with_capacity(digest * 2",
        "with_capacity()",
        "with_capacity(bytes.len() * 2)",
        "with_capacity(digest.len() * 3)",
        "with_capacity(3 * hash.len())",
        "with_capacity(digest.len() * NAMED_SCALE)",
        "with_capacity(hash.len() << NAMED_SHIFT)",
        "with_capacity(hash.len() << 2)",
        "with_capacity(hash.len() + 2)",
        "with_capacity(digest *)",
    ] {
        assert_eq!(count(source, inspect_hex_capacities), 0, "{source}");
    }
}
