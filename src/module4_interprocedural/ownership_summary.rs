use std::collections::HashMap;
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::semantic_models::{PointerDepthClassification, SemanticModel};
use crate::data_stores::interprocedural_summaries::{OwnershipTransferSummary, OwnershipTransferType};

/// 0.4.3 Ownership-Transfer Summaries for Multi-Level Pointers
/// Evaluates callees for T** ownership consumption (Idiom 2) or creation (Idiom 3) and links back to caller variables.
pub fn generate_ownership_transfer_summaries(
    tu_ast: &TranslationUnitAST,
    semantic_models: &HashMap<String, SemanticModel>,
) -> HashMap<String, OwnershipTransferSummary> {
    let mut summaries = HashMap::new();

    for func in &tu_ast.functions {
        if let Some(model) = semantic_models.get(&func.name) {
            for param in &func.parameters {
                if let Some(PointerDepthClassification::MultiLevel { depth: _ }) = model.pointer_depth_classifications.get(&param.name) {
                    let summary_key = format!("{}:{}", func.name, param.name);

                    // Check if callee frees the pointee through T** (Idiom 2)
                    if func.body_code.contains(&format!("free(*{})", param.name)) || func.body_code.contains(&format!("free( *{})", param.name)) {
                        summaries.insert(
                            summary_key,
                            OwnershipTransferSummary {
                                param_name: param.name.clone(),
                                transfer_type: OwnershipTransferType::ConsumesOwnership,
                            },
                        );
                    }
                    // Check if callee allocates through T** out-parameter (Idiom 3)
                    else if (func.body_code.contains(&format!("*{} = malloc", param.name)) || func.body_code.contains(&format!("*{} = (", param.name)))
                        && func.body_code.contains("malloc")
                    {
                        summaries.insert(
                            summary_key,
                            OwnershipTransferSummary {
                                param_name: param.name.clone(),
                                transfer_type: OwnershipTransferType::CreatesOwnership,
                            },
                        );
                    }
                }
            }
        }
    }

    summaries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module2_ast::ast_parser::{FunctionAST, ParameterAST};
    use crate::data_stores::semantic_models::MemoryPattern;

    #[test]
    fn test_idiom2_t_star_star_consumes_ownership() {
        let tu_ast = TranslationUnitAST {
            functions: vec![FunctionAST {
                name: "free_buffer".to_string(),
                return_type: "void".to_string(),
                parameters: vec![ParameterAST {
                    name: "buf".to_string(),
                    c_type: "int".to_string(),
                    pointer_depth: 2,
                    is_union: false,
                }],
                body_code: "free(*buf); *buf = NULL;".to_string(),
                call_sites: vec![],
            }],
            unions: vec![],
            structs: vec![],
        };

        let mut model = SemanticModel::new("free_buffer".to_string());
        model.pointer_depth_classifications.insert("buf".to_string(), PointerDepthClassification::MultiLevel { depth: 2 });
        model.memory_patterns.insert("buf".to_string(), MemoryPattern::Free);

        let mut models = HashMap::new();
        models.insert("free_buffer".to_string(), model);

        let summaries = generate_ownership_transfer_summaries(&tu_ast, &models);
        assert!(summaries.contains_key("free_buffer:buf"));
        assert!(matches!(
            summaries["free_buffer:buf"].transfer_type,
            OwnershipTransferType::ConsumesOwnership
        ));
    }

    #[test]
    fn test_idiom3_t_star_star_creates_ownership() {
        let tu_ast = TranslationUnitAST {
            functions: vec![FunctionAST {
                name: "alloc_buffer".to_string(),
                return_type: "void".to_string(),
                parameters: vec![ParameterAST {
                    name: "out".to_string(),
                    c_type: "int".to_string(),
                    pointer_depth: 2,
                    is_union: false,
                }],
                body_code: "*out = malloc(100);".to_string(),
                call_sites: vec![],
            }],
            unions: vec![],
            structs: vec![],
        };

        let mut model = SemanticModel::new("alloc_buffer".to_string());
        model.pointer_depth_classifications.insert("out".to_string(), PointerDepthClassification::MultiLevel { depth: 2 });
        model.memory_patterns.insert("out".to_string(), MemoryPattern::Allocate);

        let mut models = HashMap::new();
        models.insert("alloc_buffer".to_string(), model);

        let summaries = generate_ownership_transfer_summaries(&tu_ast, &models);
        assert!(summaries.contains_key("alloc_buffer:out"));
        assert!(matches!(
            summaries["alloc_buffer:out"].transfer_type,
            OwnershipTransferType::CreatesOwnership
        ));
    }
}
