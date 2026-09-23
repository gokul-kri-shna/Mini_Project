use std::collections::HashMap;
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::interprocedural_summaries::AliasingVerdict;

/// 0.4.2 Interprocedural Alias and Points-to Analysis
/// Evaluates call-site arguments to resolve Idiom 1 (Aliased &mut parameters) and Idiom 4 (Overlapping pointer slices).
pub fn analyze_interprocedural_aliasing(
    tu_ast: &TranslationUnitAST,
) -> HashMap<String, Vec<AliasingVerdict>> {
    let mut verdicts = HashMap::new();

    for func in &tu_ast.functions {
        for call in &func.call_sites {
            let mut call_verdicts = Vec::new();
            let args = &call.arguments;

            // Check every pair of arguments at this call site
            for i in 0..args.len() {
                for j in (i + 1)..args.len() {
                    let arg1 = clean_arg(&args[i]);
                    let arg2 = clean_arg(&args[j]);

                    let aliases = check_if_args_alias(&arg1, &arg2);

                    call_verdicts.push(AliasingVerdict {
                        arg1: args[i].clone(),
                        arg2: args[j].clone(),
                        aliases,
                    });
                }
            }

            let call_key = format!("{}:{}:{}", func.name, call.callee, call.line);
            verdicts.insert(call_key, call_verdicts);
        }
    }

    verdicts
}

fn clean_arg(arg: &str) -> String {
    arg.trim_start_matches('&')
        .trim_start_matches('*')
        .trim()
        .to_string()
}

fn check_if_args_alias(arg1: &str, arg2: &str) -> bool {
    // 1. Direct variable alias (e.g. &x and &x, or p and p)
    if arg1 == arg2 {
        return true;
    }

    // 2. Idiom 4: Pointer arithmetic slice overlap check (e.g. buf + 2 and buf + 3)
    let base1 = get_pointer_base(arg1);
    let base2 = get_pointer_base(arg2);

    if base1 == base2 && !base1.is_empty() {
        return true; // Overlapping slice range detected at call site!
    }

    false
}

fn get_pointer_base(arg: &str) -> String {
    if let Some(idx) = arg.find('+') {
        arg[..idx].trim().to_string()
    } else if let Some(idx) = arg.find('-') {
        arg[..idx].trim().to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module2_ast::ast_parser::{CallSiteAST, FunctionAST};

    #[test]
    fn test_aliased_mutable_parameters_detection() {
        let tu_ast = TranslationUnitAST {
            functions: vec![FunctionAST {
                name: "main".to_string(),
                return_type: "int".to_string(),
                parameters: vec![],
                body_code: "int x = 10; foo(&x, &x);".to_string(),
                call_sites: vec![CallSiteAST {
                    callee: "foo".to_string(),
                    arguments: vec!["&x".to_string(), "&x".to_string()],
                    line: 5,
                }],
            }],
            unions: vec![],
            structs: vec![],
        };

        let verdicts = analyze_interprocedural_aliasing(&tu_ast);
        let key = "main:foo:5";
        assert!(verdicts.contains_key(key));
        assert!(verdicts[key][0].aliases); // Idiom 1 detected!
    }

    #[test]
    fn test_overlapping_slice_detection() {
        let tu_ast = TranslationUnitAST {
            functions: vec![FunctionAST {
                name: "main".to_string(),
                return_type: "int".to_string(),
                parameters: vec![],
                body_code: "combine(buf + 2, buf + 3, 5);".to_string(),
                call_sites: vec![CallSiteAST {
                    callee: "combine".to_string(),
                    arguments: vec!["buf + 2".to_string(), "buf + 3".to_string(), "5".to_string()],
                    line: 12,
                }],
            }],
            unions: vec![],
            structs: vec![],
        };

        let verdicts = analyze_interprocedural_aliasing(&tu_ast);
        let key = "main:combine:12";
        assert!(verdicts.contains_key(key));
        assert!(verdicts[key][0].aliases); // Idiom 4 detected!
    }
}
