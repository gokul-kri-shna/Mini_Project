use crate::module2_ast::ast_parser::{FunctionAST, TranslationUnitAST};
use crate::data_stores::semantic_models::{SemanticModel, UnionTransform, UnresolvedUnionCase};

/// 0.3.3 Union-to-Enum Transformer
/// Performs local union transformation and flags unresolved cross-function variant cases.
pub fn transform_unions_per_function(
    func: &FunctionAST,
    tu_ast: &TranslationUnitAST,
    model: &mut SemanticModel,
) {
    for union_def in &tu_ast.unions {
        // Check if function accesses this union
        if func.body_code.contains(&union_def.name) || func.parameters.iter().any(|p| p.c_type.contains(&union_def.name)) {
            // Check if active variant can be locally resolved from tag write (e.g. u.tag = TAG_INT; u.val.i = 10;)
            let mut resolved_variant = None;

            for variant in &union_def.variants {
                if func.body_code.contains(&format!(".{}", variant.field_name)) || func.body_code.contains(&format!("->{}", variant.field_name)) {
                    resolved_variant = Some(variant.field_name.clone());
                    break;
                }
            }

            if let Some(variant) = resolved_variant {
                model.local_union_transforms.push(UnionTransform {
                    union_name: union_def.name.clone(),
                    enum_name: format!("{}Enum", capitalize(&union_def.name)),
                    local_variant_resolved: Some(variant),
                });
            } else {
                // Cross-function active variant: forward to Module 4 (Interprocedural Analysis)
                model.unresolved_union_cases.push(UnresolvedUnionCase {
                    union_name: union_def.name.clone(),
                    function_name: func.name.clone(),
                });
            }
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module2_ast::ast_parser::{UnionAST, UnionVariantAST};

    #[test]
    fn test_local_union_transformer() {
        let tu_ast = TranslationUnitAST {
            functions: vec![],
            unions: vec![UnionAST {
                name: "Data".to_string(),
                variants: vec![
                    UnionVariantAST { field_name: "i".to_string(), field_type: "int".to_string() },
                    UnionVariantAST { field_name: "f".to_string(), field_type: "float".to_string() },
                ],
                tag_field: Some("tag".to_string()),
            }],
            structs: vec![],
        };

        let func = FunctionAST {
            name: "process_data".to_string(),
            return_type: "void".to_string(),
            parameters: vec![],
            body_code: "union Data d;\n d.i = 42;".to_string(),
            call_sites: vec![],
        };

        let mut model = SemanticModel::new("process_data".to_string());
        transform_unions_per_function(&func, &tu_ast, &mut model);

        assert_eq!(model.local_union_transforms.len(), 1);
        assert_eq!(model.local_union_transforms[0].local_variant_resolved, Some("i".to_string()));
    }
}
