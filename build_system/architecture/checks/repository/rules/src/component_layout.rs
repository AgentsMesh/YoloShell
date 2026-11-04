use crate::constants::{
    BUILD_FILES, COMPONENT_LAYOUT_POLICY_MESSAGE, COMPONENT_ROOTS, DIRECTORY_SEPARATOR,
    FLAT_COMPONENT_SEPARATOR, TEST_DIRECTORIES,
};
use std::path::Path;
use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    let path = Path::new(&file.relative);
    if !path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| BUILD_FILES.contains(&name))
    {
        return;
    }
    let Some(package) = path.parent().and_then(Path::to_str) else {
        return;
    };
    if COMPONENT_ROOTS
        .iter()
        .any(|root| flattened_component(package, root))
    {
        violations.push(Violation::file(
            &file.relative,
            COMPONENT_LAYOUT_POLICY_MESSAGE,
        ));
    }
}

fn flattened_component(package: &str, root: &str) -> bool {
    let Some((parent, family)) = root.rsplit_once(DIRECTORY_SEPARATOR) else {
        return false;
    };
    let Some(relative) = package
        .strip_prefix(parent)
        .and_then(|relative| relative.strip_prefix(DIRECTORY_SEPARATOR))
    else {
        return false;
    };
    let mut components = relative.split(DIRECTORY_SEPARATOR);
    let Some(component) = components.next() else {
        return false;
    };
    if component == family {
        return components.any(|component| {
            component.contains(FLAT_COMPONENT_SEPARATOR) && !TEST_DIRECTORIES.contains(&component)
        });
    }
    component
        .split_once(FLAT_COMPONENT_SEPARATOR)
        .is_some_and(|(prefix, _)| prefix == family)
}
