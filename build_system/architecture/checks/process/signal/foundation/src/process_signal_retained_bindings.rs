use crate::process_signal_retained_dataflow;
use crate::process_signal_retained_receiver;
use crate::process_signal_syntax;
use std::collections::BTreeSet;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_path_analysis::aliases;
use yolo_architecture_process_signal_path_analysis::paths::Resolution;
use yolo_architecture_process_signal_path_analysis::types::{
    RetainedFields, RetainedTypes, retained_fields, type_names, type_names_with,
};
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::Token;

const MEMBER_ACCESS: &str = ".";
const RECEIVER_MEMBER_OFFSET: usize = 2;
pub struct Context {
    canonical_names: BTreeSet<String>,
    types: RetainedTypes,
    fields: RetainedFields,
}
impl Context {
    pub fn new(source: &RustSource) -> Self {
        let types = type_names(&source.tokens);
        Self::with_types(source, types)
    }

    pub fn with_resolution(source: &RustSource, external: &Resolution) -> Self {
        let types = type_names_with(&source.tokens, external);
        Self::with_types(source, types)
    }

    fn with_types(source: &RustSource, types: RetainedTypes) -> Self {
        let canonical_names = aliases::exact_names(&source.tokens);
        let fields = retained_fields(&source.tokens, &types);
        Self {
            canonical_names,
            types,
            fields,
        }
    }

    pub fn is_associated(&self, tokens: &[Token], index: usize) -> bool {
        process_signal_syntax::qualified_reference(tokens, index)
            .is_some_and(|(qualifier, _)| self.canonical_names.contains(qualifier))
    }

    pub fn canonical_name<'a>(&self, name: &'a str) -> Option<&'a str> {
        self.canonical_names
            .contains(name)
            .then_some(contract::RETAINED_EXACT_PROCESS)
    }

    pub fn bindings(&self, source: &RustSource, function: &RustFunction) -> BTreeSet<String> {
        process_signal_retained_dataflow::bindings(
            &source.tokens,
            function,
            &self.types,
            &self.fields,
        )
    }

    pub fn receiver_is_retained(
        &self,
        tokens: &[Token],
        index: usize,
        function: &RustFunction,
        bindings: &BTreeSet<String>,
    ) -> bool {
        if index < RECEIVER_MEMBER_OFFSET || !tokens[index - 1].symbol(MEMBER_ACCESS) {
            return false;
        }
        process_signal_retained_receiver::range(tokens, function, index).any(|receiver| {
            identifier(&tokens[receiver]).is_some_and(|name| {
                bindings.contains(name)
                    || self.types.contains(&tokens, receiver)
                    || name == contract::SELF_RECEIVER
                        && function
                            .owner
                            .as_deref()
                            .is_some_and(|owner| self.types.contains_name(owner))
                    || self.self_field(tokens, receiver, function, name)
            })
        })
    }

    fn self_field(
        &self,
        tokens: &[Token],
        index: usize,
        function: &RustFunction,
        name: &str,
    ) -> bool {
        function
            .owner
            .as_deref()
            .is_some_and(|owner| self.fields.contains(&(owner.to_owned(), name.to_owned())))
            && index > 1
            && tokens[index - 1].symbol(MEMBER_ACCESS)
            && tokens[index - RECEIVER_MEMBER_OFFSET].identifier(contract::SELF_RECEIVER)
    }
}
