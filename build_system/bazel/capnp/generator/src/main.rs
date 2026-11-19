//! Standard capnp compiler plugin; Bazel owns the executable and output paths.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    capnpc::codegen::CodeGenerationCommand::new()
        .output_directory(std::path::Path::new("."))
        .run(std::io::stdin())?;
    Ok(())
}
