pub mod ownership_infer;
pub mod rust_codegen;

pub use ownership_infer::{infer_ownership_types, FunctionOwnershipMap};
pub use rust_codegen::synthesize_rust_code;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::semantic_models::SemanticModel;
use crate::data_stores::interprocedural_summaries::InterproceduralSummaries;

#[derive(Debug, Clone)]
pub struct CodegenResult {
    pub generated_rust_code: String,
    pub output_file_path: String,
    pub ownership_maps: HashMap<String, FunctionOwnershipMap>,
}

/// Primary entry point for Module 5: Performs Ownership Classification & Rust Code Generation.
pub fn generate_rust_code(
    tu_ast: &TranslationUnitAST,
    semantic_models: &HashMap<String, SemanticModel>,
    summaries: &InterproceduralSummaries,
    output_path: &Path,
) -> Result<CodegenResult, String> {
    // 0.5.1 Ownership Classification
    let ownership_maps = infer_ownership_types(tu_ast, semantic_models, summaries);

    // 0.5.2 Rust Code Synthesis
    let generated_rust_code = synthesize_rust_code(tu_ast, &ownership_maps, summaries);

    // Write generated candidate Rust code to file (e.g. output.rs)
    fs::write(output_path, &generated_rust_code)
        .map_err(|err| format!("Failed to write candidate Rust output to {:?}: {}", output_path, err))?;

    Ok(CodegenResult {
        generated_rust_code,
        output_file_path: output_path.to_string_lossy().to_string(),
        ownership_maps,
    })
}
