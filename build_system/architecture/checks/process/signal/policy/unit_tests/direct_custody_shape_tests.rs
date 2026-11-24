use super::*;
use yolo_architecture_syntax::targeted_tokens::Kind;

#[test]
fn direct_custody_call_requires_each_receiver_call_shape_token() {
    for site in DIRECT_TERMINATION_SITES {
        let body = format!(
            "fn {}() {{ {}.{}(); }}",
            site.site.function,
            site.receiver,
            contract::REQUEST_DIRECT_TERMINATION
        );
        let text = site.site.owner.map_or_else(
            || body.clone(),
            |owner| format!("impl {owner} {{ {body} }}"),
        );
        let file = SourceFile {
            absolute: site.site.path.into(),
            relative: site.site.path.into(),
            text: text.clone(),
        };
        let source = RustSource::parse(&text);
        let call = source
            .tokens
            .iter()
            .rposition(|token| token.identifier(contract::REQUEST_DIRECT_TERMINATION))
            .unwrap();
        assert!(call_matches(&file, &source, call, site));
        for index in [call - 2, call - 1, call + 1] {
            let mut changed = RustSource::parse(&text);
            changed.tokens[index].kind = Kind::Number(0);
            assert!(!call_matches(&file, &changed, call, site), "{index}");
        }
    }
}
