use crate::module2_ast::ast_parser::FunctionAST;
use crate::data_stores::semantic_models::{MemoryPattern, SemanticModel, UsePoint};

/// 0.3.1 Base Semantic Analyser
/// Constructs Symbol Table, Def-Use Chains, and Memory Access Classifications for a single function.
pub fn analyze_function_base(func: &FunctionAST) -> SemanticModel {
    let mut model = SemanticModel::new(func.name.clone());

    // 1. Populate Symbol Table from parameters
    for param in &func.parameters {
        let type_desc = format!("{}{}", param.c_type, "*".repeat(param.pointer_depth));
        model.symbol_table.insert(param.name.clone(), type_desc);
    }

    // 2. Scan body code for local variable declarations and usage
    let lines: Vec<&str> = func.body_code.lines().collect();

    for (idx, &line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();

        // Detect local variable declarations: e.g. "int *px = &x;" or "int x = 10;"
        if (trimmed.starts_with("int ") || trimmed.starts_with("char ") || trimmed.starts_with("float ") || trimmed.starts_with("double ") || trimmed.starts_with("void ")) && trimmed.contains('=') {
            let left = trimmed.split('=').next().unwrap().trim();
            let parts: Vec<&str> = left.split_whitespace().collect();
            if parts.len() >= 2 {
                let var_name = parts.last().unwrap().trim_matches('*').to_string();
                let var_type = parts[..parts.len() - 1].join(" ");
                model.symbol_table.insert(var_name, var_type);
            }
        }

        // Def-Use chains & Memory Access Pattern Classification
        for var_name in model.symbol_table.keys() {
            if trimmed.contains(var_name) {
                // Record usage point
                model.def_use_chains.entry(var_name.clone()).or_default().push(UsePoint {
                    line: line_num,
                    column: trimmed.find(var_name).unwrap_or(0) + 1,
                });

                // Classify Memory Access Pattern
                if trimmed.contains(&format!("free({})", var_name)) || trimmed.contains(&format!("free(*{})", var_name)) {
                    model.memory_patterns.insert(var_name.clone(), MemoryPattern::Free);
                } else if trimmed.contains(&format!("malloc(")) && trimmed.contains(var_name) {
                    model.memory_patterns.insert(var_name.clone(), MemoryPattern::Allocate);
                } else if trimmed.contains(&format!("*{}", var_name)) && trimmed.contains('=') {
                    model.memory_patterns.entry(var_name.clone()).or_insert(MemoryPattern::Write);
                } else {
                    model.memory_patterns.entry(var_name.clone()).or_insert(MemoryPattern::Read);
                }
            }
        }
    }

    model
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module2_ast::ast_parser::ParameterAST;

    #[test]
    fn test_base_semantic_analyser() {
        let func = FunctionAST {
            name: "test_fn".to_string(),
            return_type: "void".to_string(),
            parameters: vec![ParameterAST {
                name: "ptr".to_string(),
                c_type: "int".to_string(),
                pointer_depth: 1,
                is_union: false,
            }],
            body_code: "int x = 10;\n*ptr = x;\nfree(ptr);".to_string(),
            call_sites: vec![],
        };

        let model = analyze_function_base(&func);
        assert!(model.symbol_table.contains_key("ptr"));
        assert!(model.symbol_table.contains_key("x"));
        assert!(matches!(model.memory_patterns.get("ptr"), Some(MemoryPattern::Free)));
    }
}
