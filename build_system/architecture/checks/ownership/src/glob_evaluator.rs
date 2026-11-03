use crate::constants::EXCLUDE_ATTRIBUTE;
use crate::model::{CallExpression, Evaluation, Expression};
use crate::{Repository, patterns};

impl Repository {
    pub(crate) fn evaluate_glob(
        &self,
        document: &str,
        package: &str,
        line: usize,
        call: &CallExpression,
    ) -> Evaluation {
        let patterns = call
            .positional
            .first()
            .and_then(|value| self.string_values(document, line, value));
        let excluded = call.attributes.get(EXCLUDE_ATTRIBUTE).map_or_else(
            || Some(Vec::new()),
            |value| self.string_values(document, line, value),
        );
        let (Some(patterns), Some(excluded)) = (patterns, excluded) else {
            return Evaluation {
                unresolved: true,
                ..Evaluation::default()
            };
        };
        let paths = self
            .rust_sources
            .iter()
            .filter(|source| {
                self.source_package(source).as_deref() == Some(package) && {
                    let path = source[package.len()..].trim_start_matches('/');
                    patterns
                        .iter()
                        .any(|pattern| patterns::matches(path, pattern))
                        && !excluded
                            .iter()
                            .any(|pattern| patterns::matches(path, pattern))
                }
            })
            .cloned()
            .collect();
        Evaluation {
            paths,
            ..Evaluation::default()
        }
    }

    fn string_values(
        &self,
        document: &str,
        line: usize,
        expression: &Expression,
    ) -> Option<Vec<String>> {
        match expression {
            Expression::String(value) => Some(vec![value.clone()]),
            Expression::Add(values) | Expression::List(values) => {
                let mut result = Vec::new();
                for value in values {
                    result.extend(self.string_values(document, line, value)?);
                }
                Some(result)
            }
            Expression::Name(name) => self
                .documents
                .get(document)
                .and_then(|source| source.assignments.get(name))
                .and_then(|values| values.iter().rev().find(|item| item.line <= line))
                .and_then(|item| self.string_values(document, item.line, &item.expression)),
            _ => None,
        }
    }
}
