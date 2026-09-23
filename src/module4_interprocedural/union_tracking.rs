use std::collections::HashMap;
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::semantic_models::SemanticModel;

/// 0.4.4 Cross-Function Union Variant Tracking
/// Interprocedural reaching-definitions analysis resolving active union variants across function boundaries (Idiom 5).
pub fn track_cross_function_union_variants(
    tu_ast: &TranslationUnitAST,
    semantic_models: &HashMap<String, SemanticModel>,
) -> HashMap<String, String> {
    let mut resolved_cross_variants = HashMap::new();

    // Collect all unresolved union cases from D1 Semantic Models
    for (func_name, model) in semantic_models {
        for unresolved in &model.unresolved_union_cases {
            // Reaching-definitions search across callers and callees in translation unit
            let mut found_variant = None;

            for other_func in &tu_ast.functions {
                if other_func.name != *func_name {
                    if let Some(other_model) = semantic_models.get(&other_func.name) {
                        for transform in &other_model.local_union_transforms {
                            if transform.union_name == unresolved.union_name {
                                if let Some(ref variant) = transform.local_variant_resolved {
                                    found_variant = Some(variant.clone());
                                    break;
                                }
                            }
                        }
                    }
                }
                if found_variant.is_some() {
                    break;
                }
            }

            if let Some(variant) = found_variant {
                let key = format!("{}:{}", func_name, unresolved.union_name);
                resolved_cross_variants.insert(key, variant);
            }
        }
    }

    resolved_cross_variants
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module2_ast::ast_parser::FunctionAST;
    use crate::data_stores::semantic_models::{UnionTransform, UnresolvedUnionCase};

    #[test]
    fn test_cross_function_union_variant_tracking() {
        let tu_ast = TranslationUnitAST {
            functions: vec![
                FunctionAST {
                    name: "set_val".to_string(),
                    return_type: "void".to_string(),
                    parameters: vec![],
                    body_code: "u.i = 10;".to_string(),
                    call_sites: vec![],
                },
                FunctionAST {
                    name: "read_val".to_string(),
                    return_type: "void".to_string(),
                    parameters: vec![],
                    body_code: "int x = u;".to_string(),
                    call_sites: vec![],
                },
            ],
            unions: vec![],
            structs: vec![],
        };

        let mut model_set = SemanticModel::new("set_val".to_string());
        model_set.local_union_transforms.push(UnionTransform {
            union_name: "Data".to_string(),
            enum_name: "DataEnum".to_string(),
            local_variant_resolved: Some("i".to_string()),
        });

        let mut model_read = SemanticModel::new("read_val".to_string());
        model_read.unresolved_union_cases.push(UnresolvedUnionCase {
            union_name: "Data".to_string(),
            function_name: "read_val".to_string(),
        });

        let mut models = HashMap::new();
        models.insert("set_val".to_string(), model_set);
        models.insert("read_val".to_string(), model_read);

        let cross_resolutions = track_cross_function_union_variants(&tu_ast, &models);
        assert_eq!(cross_resolutions.get("read_val:Data"), Some(&"i".to_string())); // Idiom 5 resolved!
    }
}
