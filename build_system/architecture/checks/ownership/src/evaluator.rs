use crate::constants::GLOB_FUNCTION;
use crate::model::{Evaluation, Expression};
use crate::{Repository, paths};
use std::collections::BTreeSet;

impl Repository {
    pub fn evaluate(
        &self,
        document: &str,
        package: &str,
        line: usize,
        expression: &Expression,
    ) -> Evaluation {
        let mut visited = BTreeSet::new();
        let mut result = self.evaluate_inner(document, package, line, expression, &mut visited);
        result.normalize();
        result
    }

    fn evaluate_inner(
        &self,
        document: &str,
        package: &str,
        line: usize,
        expression: &Expression,
        visited: &mut BTreeSet<String>,
    ) -> Evaluation {
        match expression {
            Expression::Add(values) | Expression::List(values) => {
                values
                    .iter()
                    .fold(Evaluation::default(), |mut result, value| {
                        result.extend(self.evaluate_inner(document, package, line, value, visited));
                        result
                    })
            }
            Expression::Call(call) if call.name == GLOB_FUNCTION => {
                self.evaluate_glob(document, package, line, call)
            }
            Expression::Name(name) => self.evaluate_name(document, package, line, name, visited),
            Expression::String(value) => self.evaluate_string(package, value, visited),
            Expression::Call(_) | Expression::Unknown => Evaluation {
                unresolved: true,
                ..Evaluation::default()
            },
        }
    }

    fn evaluate_name(
        &self,
        document: &str,
        package: &str,
        line: usize,
        name: &str,
        visited: &mut BTreeSet<String>,
    ) -> Evaluation {
        let key = format!("{document}#{name}");
        if !visited.insert(key.clone()) {
            return Evaluation::default();
        }
        let Some(source) = self.documents.get(document) else {
            return Evaluation::default();
        };
        let result = if let Some(assignment) = source.assignments.get(name).and_then(|values| {
            values
                .iter()
                .rev()
                .find(|assignment| assignment.line <= line)
        }) {
            self.evaluate_inner(
                document,
                package,
                assignment.line,
                &assignment.expression,
                visited,
            )
        } else if let Some(import) = source.imports.get(name) {
            let (import_path, symbol) = import.split_once('#').unwrap_or((import, name));
            self.evaluate_name(import_path, package, usize::MAX, symbol, visited)
        } else {
            Evaluation {
                unresolved: true,
                ..Evaluation::default()
            }
        };
        visited.remove(&key);
        result
    }

    fn evaluate_string(
        &self,
        package: &str,
        value: &str,
        visited: &mut BTreeSet<String>,
    ) -> Evaluation {
        if let Some(path) = paths::source_path(package, value) {
            return Evaluation {
                paths: vec![path],
                ..Evaluation::default()
            };
        }
        let Some((label_package, name)) = paths::label_parts(package, value) else {
            return Evaluation::default();
        };
        let label = paths::label(&label_package, &name);
        let mut result = Evaluation {
            labels: vec![label.clone()],
            ..Evaluation::default()
        };
        if let Some(group) = self.groups.get(&label) {
            let key = format!("group#{label}");
            if visited.insert(key.clone()) {
                result.extend(self.evaluate_inner(
                    &group.document,
                    &group.package,
                    group.line,
                    &group.expression,
                    visited,
                ));
                visited.remove(&key);
            }
        } else {
            result.unresolved = true;
        }
        result
    }
}
