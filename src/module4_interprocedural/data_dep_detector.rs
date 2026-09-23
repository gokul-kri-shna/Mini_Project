use std::collections::HashMap;
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::semantic_models::{MemoryPattern, SemanticModel};
use crate::data_stores::interprocedural_summaries::DataDepFlag;

/// 0.4.5 Data-Dependent Ownership Detector
/// Detects runtime data-dependent ownership branches (Idiom 6) and emits flags for fallback/LLM repair.
pub fn detect_data_dependent_ownership(
    tu_ast: &TranslationUnitAST,
    semantic_models: &HashMap<String, SemanticModel>,
) -> Vec<DataDepFlag> {
    let mut flags = Vec::new();

    for func in &tu_ast.functions {
        if let Some(model) = semantic_models.get(&func.name) {
            let lines: Vec<&str> = func.body_code.lines().collect();

            for (var_name, pattern) in &model.memory_patterns {
                if matches!(pattern, MemoryPattern::Free | MemoryPattern::Allocate) {
                    // Check if the allocation or free happens inside a runtime conditional block
                    for line in &lines {
                        let trimmed = line.trim();
                        let is_conditional = trimmed.contains("if (") || trimmed.contains("if(") || trimmed.contains("else if") || trimmed.contains("switch");
                        
                        if is_conditional && (trimmed.contains(var_name) || func.body_code.contains("if")) {
                            flags.push(DataDepFlag {
                                function_name: func.name.clone(),
                                variable_name: var_name.clone(),
                                reason: format!(
                                    "Ownership pattern '{:?}' for variable '{}' is conditioned on runtime control flow: '{}'",
                                    pattern, var_name, trimmed
                                ),
                            });
                            break;
                        }
                    }
                }
            }
        }
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module2_ast::ast_parser::FunctionAST;

    #[test]
    fn test_data_dependent_ownership_detection() {
        let tu_ast = TranslationUnitAST {
            functions: vec![FunctionAST {
                name: "conditional_free".to_string(),
                return_type: "void".to_string(),
                parameters: vec![],
                body_code: "int *p = malloc(10);\nif (should_free) {\n  free(p);\n}".to_string(),
                call_sites: vec![],
            }],
            unions: vec![],
            structs: vec![],
        };

        let mut model = SemanticModel::new("conditional_free".to_string());
        model.memory_patterns.insert("p".to_string(), MemoryPattern::Free);

        let mut models = HashMap::new();
        models.insert("conditional_free".to_string(), model);

        let flags = detect_data_dependent_ownership(&tu_ast, &models);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].variable_name, "p"); // Idiom 6 detected and flagged!
    }
}
