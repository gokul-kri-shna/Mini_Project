use crate::module2_ast::ast_parser::FunctionAST;
use crate::data_stores::semantic_models::{PointerDepthClassification, SemanticModel};

/// 0.3.2 Multi-Level Pointer Chain Analyser
/// Evaluates T** and deeper pointer chains for per-variable, per-indirection level behavior.
pub fn analyze_pointer_chains(func: &FunctionAST, model: &mut SemanticModel) {
    for param in &func.parameters {
        if param.pointer_depth >= 2 {
            model.pointer_depth_classifications.insert(
                param.name.clone(),
                PointerDepthClassification::MultiLevel {
                    depth: param.pointer_depth,
                },
            );
        } else {
            model.pointer_depth_classifications.insert(
                param.name.clone(),
                PointerDepthClassification::SingleLevel,
            );
        }
    }

    // Inspect body for multi-level indirection accesses (e.g. **a = temp; free(*a);)
    for line in func.body_code.lines() {
        let trimmed = line.trim();
        for (var_name, classif) in &model.pointer_depth_classifications {
            if let PointerDepthClassification::MultiLevel { depth: _ } = classif {
                let double_deref = format!("**{}", var_name);
                let single_deref = format!("*{}", var_name);

                if trimmed.contains(&format!("free({})", single_deref)) {
                    // Idiom 2 precursor: Function frees pointee through T**
                    model.memory_patterns.insert(var_name.clone(), crate::data_stores::semantic_models::MemoryPattern::Free);
                } else if trimmed.contains(&double_deref) && trimmed.contains('=') {
                    model.memory_patterns.insert(var_name.clone(), crate::data_stores::semantic_models::MemoryPattern::Write);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module2_ast::ast_parser::ParameterAST;

    #[test]
    fn test_multi_level_pointer_chain_analyser() {
        let func = FunctionAST {
            name: "swap".to_string(),
            return_type: "void".to_string(),
            parameters: vec![
                ParameterAST {
                    name: "a".to_string(),
                    c_type: "int".to_string(),
                    pointer_depth: 2,
                    is_union: false,
                },
                ParameterAST {
                    name: "b".to_string(),
                    c_type: "int".to_string(),
                    pointer_depth: 2,
                    is_union: false,
                },
            ],
            body_code: "int *temp = *a;\n*a = *b;\n*b = temp;".to_string(),
            call_sites: vec![],
        };

        let mut model = SemanticModel::new("swap".to_string());
        analyze_pointer_chains(&func, &mut model);

        assert!(matches!(
            model.pointer_depth_classifications.get("a"),
            Some(PointerDepthClassification::MultiLevel { depth: 2 })
        ));
    }
}
