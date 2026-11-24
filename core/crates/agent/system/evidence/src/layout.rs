use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};

/// Shape of a provider transcript path relative to its data home.
///
/// `root_component` must be the first path component (for example `projects`),
/// and `relative_components` is the exact depth below the data home.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvidenceLayout {
    root_component: &'static str,
    relative_components: usize,
}

impl EvidenceLayout {
    pub const fn new(root_component: &'static str, relative_components: usize) -> Self {
        Self {
            root_component,
            relative_components,
        }
    }

    pub const fn root_component(self) -> &'static str {
        self.root_component
    }

    pub const fn relative_components(self) -> usize {
        self.relative_components
    }

    pub(crate) fn relative_names(self, root: &Path, path: &Path) -> Option<Vec<CString>> {
        if !root.is_absolute()
            || root.components().any(|part| part == Component::ParentDir)
            || self.relative_components == 0
        {
            return None;
        }
        let relative = path.strip_prefix(root).ok()?;
        let names = relative
            .components()
            .map(|part| match part {
                Component::Normal(name) => CString::new(name.as_bytes()).ok(),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        (names.len() == self.relative_components
            && names
                .first()
                .is_some_and(|name| name.as_bytes() == self.root_component.as_bytes()))
        .then_some(names)
    }
}
