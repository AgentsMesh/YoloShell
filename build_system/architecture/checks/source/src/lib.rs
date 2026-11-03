pub mod brand_rules;
pub mod constants;
pub mod line_rules;
mod rust_test_declarations;
pub mod source_policy;
mod web_vendor;

#[cfg(test)]
pub fn inspect(
    root: &std::path::Path,
) -> std::io::Result<Vec<yolo_architecture_foundation::model::Violation>> {
    let files = yolo_architecture_test_support::load(root)?;
    let mut violations = Vec::new();
    for file in &files {
        line_rules::inspect(file, &mut violations);
        source_policy::inspect(file, &mut violations);
    }
    Ok(violations)
}
