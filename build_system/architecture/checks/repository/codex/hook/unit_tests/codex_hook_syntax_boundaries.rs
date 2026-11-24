use super::*;

#[test]
fn watch_mutations_require_a_named_sender_method_and_call_delimiter() {
    for (text, expected) in [
        ("self.authority_tx.send", None),
        ("self.authority_tx.send()", Some("send")),
        ("self.authority_tx.send_modify()", Some("send_modify")),
        ("self.authority_tx.\"send\"()", None),
        ("self.authority_tx.unknown()", None),
        ("self.other.send()", None),
    ] {
        let tokens = tokenize(text);
        let method_index = tokens.iter().rposition(|token| token.symbol(".")).unwrap() + NEXT;
        assert_eq!(mutation_method(&tokens, method_index), expected, "{text}");
    }
}

#[test]
fn exposing_sender_or_unrecognized_sender_methods_is_detected() {
    for (text, escapes) in [
        ("self.authority_tx", true),
        ("self.authority_tx.subscribe", true),
        ("self.authority_tx.subscribe()", false),
        ("self.authority_tx.send_replace(view)", false),
        ("self.authority_tx.clone()", true),
        ("self.authority_tx.unknown()", true),
        ("authority_tx", false),
    ] {
        let tokens = tokenize(text);
        let index = tokens
            .iter()
            .position(|token| token.identifier("authority_tx"))
            .unwrap();
        assert_eq!(sender_escapes(&tokens, index), escapes, "{text}");
    }
}

#[test]
fn incomplete_or_rebound_proof_arguments_never_qualify_as_a_gated_call() {
    for (text, matches) in [
        (CODEX_AUTHORITY_PUBLISH_PROVEN, false),
        ("publish_proven(", false),
        ("publish_proven(proof", false),
        ("publish_proven(other, view)", false),
        ("publish_proven(proof.field, view)", false),
        ("publish_proven(proof)", true),
        ("publish_proven(proof, view)", true),
    ] {
        let source = RustSource::parse(text);
        assert_eq!(
            call_starts_with(&source, usize::default(), PROOF_PARAMETER),
            matches,
            "{text}"
        );
    }
}

#[test]
fn supersession_gate_requires_owned_proof_type_and_exact_argument() {
    for (proof_type, argument) in [("Other", "proof"), (SUCCESSION_PROOF, "different")] {
        let source = RustSource::parse(&format!(
            "impl {CONTROL_OWNER} {{ fn supersede_locked(&self, proof: {proof_type}) {{ self.publish_superseded({argument}); }} }}"
        ));
        let function = source.functions().next().unwrap();
        let index = source
            .tokens
            .iter()
            .position(|token| token.identifier(CODEX_AUTHORITY_PUBLISH_SUPERSEDED))
            .unwrap();
        assert!(!allowed_gated_call(
            &source,
            function,
            index,
            CODEX_AUTHORITY_PUBLISH_SUPERSEDED
        ));
    }
}

#[test]
fn undeclared_publisher_names_do_not_receive_an_implicit_contract() {
    let source = RustSource::parse("impl Other { fn future(&self) {} }");
    let function = source.functions().next().unwrap();
    assert!(!contract(&source, function));
    assert!(!allowed_gated_call(
        &source,
        function,
        usize::default(),
        "future"
    ));
}
