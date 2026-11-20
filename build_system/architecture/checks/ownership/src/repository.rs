use crate::constants::{
    ACTUAL_ATTRIBUTE, ALIAS_RULE, FILEGROUP_RULE, NAME_ATTRIBUTE, RUST_SOURCE_EXTENSION,
    SOURCES_ATTRIBUTE, STARLARK_EXTENSION,
};
use crate::model::{Document, Expression, FileGroup};
use crate::{parser, paths};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use yolo_architecture_foundation::SourceFile;

pub struct Repository {
    pub(crate) documents: BTreeMap<String, Document>,
    pub(crate) groups: BTreeMap<String, FileGroup>,
    pub(crate) packages: BTreeSet<String>,
    pub(crate) rust_sources: BTreeSet<String>,
    pub(crate) rust_source_lines: BTreeMap<String, usize>,
}

impl Repository {
    pub fn new(files: &[SourceFile]) -> Self {
        let documents = files
            .iter()
            .filter(|file| document_source(&file.relative))
            .map(parser::document)
            .map(|document| (document.path.clone(), document))
            .collect::<BTreeMap<_, _>>();
        let packages = documents
            .values()
            .filter(|document| paths::is_build_file(&document.path))
            .map(|document| document.package.clone())
            .collect();
        let rust_sources = files
            .iter()
            .filter(|file| extension(&file.relative) == Some(RUST_SOURCE_EXTENSION))
            .map(|file| file.relative.clone())
            .collect();
        let rust_source_lines = files
            .iter()
            .filter(|file| extension(&file.relative) == Some(RUST_SOURCE_EXTENSION))
            .map(|file| (file.relative.clone(), file.text.lines().count()))
            .collect();
        let groups = file_groups(&documents);
        Self {
            documents,
            groups,
            packages,
            rust_sources,
            rust_source_lines,
        }
    }

    pub fn documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.values()
    }

    pub fn document(&self, path: &str) -> Option<&Document> {
        self.documents.get(path)
    }

    pub fn groups(&self) -> impl Iterator<Item = &FileGroup> {
        self.groups.values()
    }

    pub fn group(&self, label: &str) -> Option<&FileGroup> {
        self.groups.get(label)
    }

    pub fn rust_sources(&self) -> impl Iterator<Item = &String> {
        self.rust_sources.iter()
    }

    pub fn rust_source_lines(&self, path: &str) -> Option<usize> {
        self.rust_source_lines.get(path).copied()
    }

    pub fn source_package(&self, path: &str) -> Option<String> {
        Path::new(path)
            .parent()?
            .ancestors()
            .filter_map(Path::to_str)
            .find(|parent| self.packages.contains(*parent))
            .map(str::to_owned)
    }
}

fn file_groups(documents: &BTreeMap<String, Document>) -> BTreeMap<String, FileGroup> {
    let mut groups = BTreeMap::new();
    for document in documents
        .values()
        .filter(|item| paths::is_build_file(&item.path))
    {
        for rule in document
            .rules
            .iter()
            .filter(|rule| matches!(rule.name.as_str(), FILEGROUP_RULE | ALIAS_RULE))
        {
            let Some(Expression::String(name)) = rule.attributes.get(NAME_ATTRIBUTE) else {
                continue;
            };
            let source_attribute = if rule.name == ALIAS_RULE {
                ACTUAL_ATTRIBUTE
            } else {
                SOURCES_ATTRIBUTE
            };
            let Some(expression) = rule.attributes.get(source_attribute) else {
                continue;
            };
            let label = paths::label(&document.package, name);
            groups.insert(
                label.clone(),
                FileGroup {
                    label,
                    name: name.clone(),
                    package: document.package.clone(),
                    document: document.path.clone(),
                    line: rule.line,
                    expression: expression.clone(),
                },
            );
        }
    }
    groups
}

fn document_source(path: &str) -> bool {
    paths::is_build_file(path) || extension(path) == Some(STARLARK_EXTENSION)
}

fn extension(path: &str) -> Option<&str> {
    Path::new(path).extension().and_then(|value| value.to_str())
}
