use std::collections::HashMap;
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::semantic_models::{MemoryPattern, SemanticModel};
use crate::data_stores::interprocedural_summaries::{InterproceduralSummaries, OwnershipKind, OwnershipTransferType};

#[derive(Debug, Clone)]
pub struct FunctionOwnershipMap {
    pub function_name: String,
    pub variable_ownership: HashMap<String, OwnershipKind>,
}

/// 0.5.1 Ownership Classifier
/// Infers Rust ownership types (Box<T>, &mut T, &T, Vec<T>, Option<Box<T>>) from D1 & D2.
pub fn infer_ownership_types(
    tu_ast: &TranslationUnitAST,
    semantic_models: &HashMap<String, SemanticModel>,
    summaries: &InterproceduralSummaries,
) -> HashMap<String, FunctionOwnershipMap> {
    let mut ownership_maps = HashMap::new();

    for func in &tu_ast.functions {
        let mut map = FunctionOwnershipMap {
            function_name: func.name.clone(),
            variable_ownership: HashMap::new(),
        };

        if let Some(model) = semantic_models.get(&func.name) {
            // Classify parameters
            for param in &func.parameters {
                let kind = classify_parameter(
                    &func.name,
                    &param.name,
                    param.pointer_depth,
                    model,
                    summaries,
                );
                map.variable_ownership.insert(param.name.clone(), kind);
            }

            // Classify local variables
            for (var_name, _var_type) in &model.symbol_table {
                if !map.variable_ownership.contains_key(var_name) {
                    let kind = classify_local_var(
                        &func.name,
                        var_name,
                        model,
                        summaries,
                    );
                    map.variable_ownership.insert(var_name.clone(), kind);
                }
            }
        }

        ownership_maps.insert(func.name.clone(), map);
    }

    ownership_maps
}

fn classify_parameter(
    func_name: &str,
    param_name: &str,
    pointer_depth: usize,
    model: &SemanticModel,
    summaries: &InterproceduralSummaries,
) -> OwnershipKind {
    // Check Idiom 6: Data-dependent runtime ownership -> Option<Box<T>> fallback
    for flag in &summaries.data_dependent_ownership_flags {
        if flag.function_name == func_name && flag.variable_name == param_name {
            return OwnershipKind::ConservativeFallback;
        }
    }

    // Check Idiom 2 & 3: T** Ownership transfer summaries
    let summary_key = format!("{}:{}", func_name, param_name);
    if let Some(transfer) = summaries.ownership_transfer_summaries.get(&summary_key) {
        match transfer.transfer_type {
            OwnershipTransferType::CreatesOwnership => return OwnershipKind::UniqueHeapOwner,
            OwnershipTransferType::ConsumesOwnership => return OwnershipKind::UniqueHeapOwner,
            OwnershipTransferType::None => {}
        }
    }

    // Standard classification based on pointer depth and memory access pattern
    if pointer_depth == 0 {
        return OwnershipKind::SharedBorrow;
    }

    let pattern = model.memory_patterns.get(param_name);
    match pattern {
        Some(MemoryPattern::Allocate) => OwnershipKind::UniqueHeapOwner,
        Some(MemoryPattern::Free) => OwnershipKind::UniqueHeapOwner,
        Some(MemoryPattern::Write) => OwnershipKind::MutableBorrow,
        Some(MemoryPattern::Read) | None => {
            if pointer_depth >= 2 {
                OwnershipKind::MutableBorrow
            } else {
                OwnershipKind::SharedBorrow
            }
        }
    }
}

fn classify_local_var(
    func_name: &str,
    var_name: &str,
    model: &SemanticModel,
    summaries: &InterproceduralSummaries,
) -> OwnershipKind {
    for flag in &summaries.data_dependent_ownership_flags {
        if flag.function_name == func_name && flag.variable_name == var_name {
            return OwnershipKind::ConservativeFallback;
        }
    }

    let pattern = model.memory_patterns.get(var_name);
    match pattern {
        Some(MemoryPattern::Allocate) | Some(MemoryPattern::Free) => OwnershipKind::UniqueHeapOwner,
        Some(MemoryPattern::Write) => OwnershipKind::MutableBorrow,
        _ => OwnershipKind::SharedBorrow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module2_ast::ast_parser::{FunctionAST, ParameterAST};
    use crate::data_stores::semantic_models::PointerDepthClassification;

    #[test]
    fn test_ownership_infer_t_star_star() {
        let tu_ast = TranslationUnitAST {
            functions: vec![FunctionAST {
                name: "swap".to_string(),
                return_type: "void".to_string(),
                parameters: vec![ParameterAST {
                    name: "a".to_string(),
                    c_type: "int".to_string(),
                    pointer_depth: 2,
                    is_union: false,
                }],
                body_code: "*a = NULL;".to_string(),
                call_sites: vec![],
            }],
            unions: vec![],
            structs: vec![],
        };

        let mut model = SemanticModel::new("swap".to_string());
        model.pointer_depth_classifications.insert("a".to_string(), PointerDepthClassification::MultiLevel { depth: 2 });
        model.memory_patterns.insert("a".to_string(), MemoryPattern::Write);

        let mut models = HashMap::new();
        models.insert("swap".to_string(), model);

        let summaries = InterproceduralSummaries::new();

        let ownership_maps = infer_ownership_types(&tu_ast, &models, &summaries);
        assert!(ownership_maps.contains_key("swap"));
        assert!(matches!(
            ownership_maps["swap"].variable_ownership["a"],
            OwnershipKind::MutableBorrow
        ));
    }
}
