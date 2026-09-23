pub mod base_analyser;
pub mod pointer_chain;
pub mod union_transformer;

pub use base_analyser::analyze_function_base;
pub use pointer_chain::analyze_pointer_chains;
pub use union_transformer::transform_unions_per_function;

use std::collections::HashMap;
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::semantic_models::SemanticModel;

/// Primary entry point for Module 3: Performs per-function semantic analysis and populates D1 Semantic Models.
pub fn analyze_translation_unit_semantics(
    tu_ast: &TranslationUnitAST,
) -> HashMap<String, SemanticModel> {
    let mut semantic_models = HashMap::new();

    for func in &tu_ast.functions {
        // 0.3.1 Base Semantic Analysis (Symbol Tables, Def-Use, Alias, Access Patterns)
        let mut model = analyze_function_base(func);

        // 0.3.2 Multi-Level Pointer Chain Analysis (T** indirection depths)
        analyze_pointer_chains(func, &mut model);

        // 0.3.3 Union-to-Enum Transformation
        transform_unions_per_function(func, tu_ast, &mut model);

        semantic_models.insert(func.name.clone(), model);
    }

    semantic_models
}
