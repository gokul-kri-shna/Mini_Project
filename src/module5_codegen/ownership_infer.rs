use std::collections::{HashMap, HashSet};
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::semantic_models::{MemoryPattern, SemanticModel};
use crate::data_stores::interprocedural_summaries::{InterproceduralSummaries, OwnershipKind, OwnershipTransferType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LValueStoreStatus {
    ReadOnly,   // only r-value accesses (expressions, return, conditions)
    OuterStore, // *param = ... or free(*param)
    InnerStore, // **param = ... or (*param)[i] = ...
    BothStore,  // both levels stored
}

#[derive(Debug, Clone)]
pub struct FunctionOwnershipMap {
    pub function_name: String,
    pub variable_ownership: HashMap<String, OwnershipKind>,
    pub aliased_param_pairs: HashSet<(String, String)>,
    pub is_overlapping_slice: bool,
    pub param_lvalue_status: HashMap<String, LValueStoreStatus>,
}

/// Intra-procedural AST/CFG traversal pass to check the l-value store status of dereferenced parameters.
pub fn analyze_lvalue_store_status(func_body: &str, param_name: &str) -> LValueStoreStatus {
    let mut outer_store = false;
    let mut inner_store = false;

    for line in func_body.lines() {
        let trimmed = line.trim();
        // Skip comments and comparisons like ==, !=, <=, >=
        if trimmed.starts_with("//") {
            continue;
        }
        // Skip variable declaration statements (e.g. int *inner = &value;)
        let is_declaration = trimmed.starts_with("int ")
            || trimmed.starts_with("char ")
            || trimmed.starts_with("float ")
            || trimmed.starts_with("double ")
            || trimmed.starts_with("long ")
            || trimmed.starts_with("short ")
            || trimmed.starts_with("unsigned ")
            || trimmed.starts_with("void ")
            || trimmed.starts_with("size_t ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("union ")
            || trimmed.starts_with("let ");
        if is_declaration {
            continue;
        }
        let is_comparison = trimmed.contains("==") || trimmed.contains("!=");

        // Check inner store: **param = ..., **param += ..., (*param)[...] = ...
        let inner_assign = trimmed.contains(&format!("**{}", param_name)) && trimmed.contains('=') && !is_comparison;
        let index_assign = trimmed.contains(&format!("(*{})[", param_name)) && trimmed.contains('=') && !is_comparison;
        if inner_assign || index_assign {
            inner_store = true;
        }

        // Check outer store: *param = ..., free(*param), realloc(*param)
        let outer_assign = trimmed.contains(&format!("*{}", param_name))
            && !trimmed.contains(&format!("**{}", param_name))
            && trimmed.contains('=')
            && !is_comparison;
        let outer_free = trimmed.contains(&format!("free(*{})", param_name))
            || trimmed.contains(&format!("free( *{})", param_name));
        let outer_realloc = trimmed.contains(&format!("realloc(*{})", param_name));
        if outer_assign || outer_free || outer_realloc {
            outer_store = true;
        }
    }

    match (outer_store, inner_store) {
        (false, false) => LValueStoreStatus::ReadOnly,
        (true, false) => LValueStoreStatus::OuterStore,
        (false, true) => LValueStoreStatus::InnerStore,
        (true, true) => LValueStoreStatus::BothStore,
    }
}

/// Bridges Module 4's alias data (keyed by "caller:callee:line", storing raw call-site
/// argument text) to what this module needs: for each function, the set of its OWN
/// parameter-name pairs verified to alias at some call site somewhere in the program.
fn compute_verified_aliased_parameter_pairs(
    tu_ast: &TranslationUnitAST,
    summaries: &InterproceduralSummaries,
) -> HashMap<String, HashSet<(String, String)>> {
    let mut result: HashMap<String, HashSet<(String, String)>> = HashMap::new();

    let param_names: HashMap<&str, Vec<&str>> = tu_ast
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.parameters.iter().map(|p| p.name.as_str()).collect()))
        .collect();

    for caller_func in &tu_ast.functions {
        for call in &caller_func.call_sites {
            let key = format!("{}:{}:{}", caller_func.name, call.callee, call.line);
            let Some(verdicts) = summaries.alias_verdicts.get(&key) else { continue };
            let Some(callee_params) = param_names.get(call.callee.as_str()) else { continue };

            let n = call.arguments.len();
            let mut idx = 0;
            for i in 0..n {
                for j in (i + 1)..n {
                    if let Some(v) = verdicts.get(idx) {
                        if v.aliases && i < callee_params.len() && j < callee_params.len() {
                            result
                                .entry(call.callee.clone())
                                .or_default()
                                .insert((callee_params[i].to_string(), callee_params[j].to_string()));
                        }
                    }
                    idx += 1;
                }
            }
        }
    }

    result
}

fn is_involved_in_verified_aliasing(
    func_name: &str,
    param_name: &str,
    aliased_pairs: &HashMap<String, HashSet<(String, String)>>,
) -> bool {
    aliased_pairs
        .get(func_name)
        .map(|pairs| pairs.iter().any(|(a, b)| a == param_name || b == param_name))
        .unwrap_or(false)
}

fn detect_overlapping_slices_for_function(
    func_name: &str,
    tu_ast: &TranslationUnitAST,
    summaries: &InterproceduralSummaries,
) -> bool {
    for caller_func in &tu_ast.functions {
        for call in &caller_func.call_sites {
            if call.callee == func_name {
                let key = format!("{}:{}:{}", caller_func.name, call.callee, call.line);
                if let Some(verdicts) = summaries.alias_verdicts.get(&key) {
                    if verdicts.iter().any(|v| v.aliases && (v.arg1.contains('+') || v.arg2.contains('+'))) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// 0.5.1 Ownership Classifier
/// Infers Rust ownership types (Box<T>, &mut T, &T, Vec<T>, Option<Box<T>>) from D1 & D2.
pub fn infer_ownership_types(
    tu_ast: &TranslationUnitAST,
    semantic_models: &HashMap<String, SemanticModel>,
    summaries: &InterproceduralSummaries,
) -> HashMap<String, FunctionOwnershipMap> {
    let mut ownership_maps = HashMap::new();
    let aliased_pairs = compute_verified_aliased_parameter_pairs(tu_ast, summaries);

    for func in &tu_ast.functions {
        let is_slice_overlap = detect_overlapping_slices_for_function(&func.name, tu_ast, summaries);
        let pairs_for_func = aliased_pairs.get(&func.name).cloned().unwrap_or_default();
        let mut param_lval_status = HashMap::new();

        let mut map = FunctionOwnershipMap {
            function_name: func.name.clone(),
            variable_ownership: HashMap::new(),
            aliased_param_pairs: pairs_for_func,
            is_overlapping_slice: is_slice_overlap,
            param_lvalue_status: HashMap::new(),
        };

        if let Some(model) = semantic_models.get(&func.name) {
            // Classify parameters
            for param in &func.parameters {
                let lval_status = analyze_lvalue_store_status(&func.body_code, &param.name);
                param_lval_status.insert(param.name.clone(), lval_status);

                let kind = classify_parameter(
                    &func.name,
                    &param.name,
                    param.pointer_depth,
                    model,
                    summaries,
                    &aliased_pairs,
                    is_slice_overlap,
                    lval_status,
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
                        &aliased_pairs,
                    );
                    map.variable_ownership.insert(var_name.clone(), kind);
                }
            }
        }

        map.param_lvalue_status = param_lval_status;
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
    aliased_pairs: &HashMap<String, HashSet<(String, String)>>,
    is_slice_overlap: bool,
    lval_status: LValueStoreStatus,
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
            OwnershipTransferType::CreatesOwnership | OwnershipTransferType::ConsumesOwnership => {
                return OwnershipKind::UniqueHeapOwner;
            }
            OwnershipTransferType::None => {}
        }
    }

    // Standard classification based on pointer depth and memory access pattern
    if pointer_depth == 0 {
        return OwnershipKind::SharedBorrow;
    }

    // Idiom 4: Overlapping slices in arrays
    if is_slice_overlap {
        return OwnershipKind::HeapArray;
    }

    // Intra-procedural mutability guard:
    // If *param and **param only appear in r-value positions and are never written to,
    // infer SharedBorrow (which maps to &T or &&T).
    if lval_status == LValueStoreStatus::ReadOnly {
        return OwnershipKind::SharedBorrow;
    }

    let pattern = model.memory_patterns.get(param_name);
    let aliases_verified = is_involved_in_verified_aliasing(func_name, param_name, aliased_pairs);

    match pattern {
        Some(MemoryPattern::Allocate) | Some(MemoryPattern::Free) => OwnershipKind::UniqueHeapOwner,
        // Idiom 1: Aliased parameters cannot soundly be independent &mut T
        Some(MemoryPattern::Write) if aliases_verified => OwnershipKind::ConservativeFallback,
        Some(MemoryPattern::Write) => OwnershipKind::MutableBorrow,
        Some(MemoryPattern::Read) | None => OwnershipKind::SharedBorrow,
    }
}

fn classify_local_var(
    func_name: &str,
    var_name: &str,
    model: &SemanticModel,
    summaries: &InterproceduralSummaries,
    aliased_pairs: &HashMap<String, HashSet<(String, String)>>,
) -> OwnershipKind {
    for flag in &summaries.data_dependent_ownership_flags {
        if flag.function_name == func_name && flag.variable_name == var_name {
            return OwnershipKind::ConservativeFallback;
        }
    }

    let pattern = model.memory_patterns.get(var_name);
    let aliases_verified = is_involved_in_verified_aliasing(func_name, var_name, aliased_pairs);

    match pattern {
        Some(MemoryPattern::Allocate) | Some(MemoryPattern::Free) => OwnershipKind::UniqueHeapOwner,
        Some(MemoryPattern::Write) if aliases_verified => OwnershipKind::ConservativeFallback,
        Some(MemoryPattern::Write) => OwnershipKind::MutableBorrow,
        _ => OwnershipKind::SharedBorrow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module2_ast::ast_parser::{CallSiteAST, FunctionAST, ParameterAST};
    use crate::data_stores::semantic_models::PointerDepthClassification;
    use crate::data_stores::interprocedural_summaries::AliasingVerdict;

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

    #[test]
    fn test_idiom1_verified_alias_from_call_site() {
        let tu_ast = TranslationUnitAST {
            functions: vec![
                FunctionAST {
                    name: "main".to_string(),
                    return_type: "int".to_string(),
                    parameters: vec![],
                    body_code: "update_twin_counters(handle, handle);".to_string(),
                    call_sites: vec![CallSiteAST {
                        callee: "update_twin_counters".to_string(),
                        arguments: vec!["handle".to_string(), "handle".to_string()],
                        line: 10,
                    }],
                },
                FunctionAST {
                    name: "update_twin_counters".to_string(),
                    return_type: "void".to_string(),
                    parameters: vec![
                        ParameterAST { name: "counter_a".to_string(), c_type: "int".to_string(), pointer_depth: 1, is_union: false },
                        ParameterAST { name: "counter_b".to_string(), c_type: "int".to_string(), pointer_depth: 1, is_union: false },
                    ],
                    body_code: "*counter_a += 1; *counter_b += 5;".to_string(),
                    call_sites: vec![],
                },
            ],
            unions: vec![],
            structs: vec![],
        };

        let mut model = SemanticModel::new("update_twin_counters".to_string());
        model.memory_patterns.insert("counter_a".to_string(), MemoryPattern::Write);
        model.memory_patterns.insert("counter_b".to_string(), MemoryPattern::Write);

        let mut models = HashMap::new();
        models.insert("update_twin_counters".to_string(), model);
        models.insert("main".to_string(), SemanticModel::new("main".to_string()));

        let mut summaries = InterproceduralSummaries::new();
        summaries.alias_verdicts.insert(
            "main:update_twin_counters:10".to_string(),
            vec![AliasingVerdict { arg1: "handle".to_string(), arg2: "handle".to_string(), aliases: true }],
        );

        let ownership_maps = infer_ownership_types(&tu_ast, &models, &summaries);
        assert!(matches!(
            ownership_maps["update_twin_counters"].variable_ownership["counter_a"],
            OwnershipKind::ConservativeFallback
        ));
        assert!(matches!(
            ownership_maps["update_twin_counters"].variable_ownership["counter_b"],
            OwnershipKind::ConservativeFallback
        ));
        assert!(ownership_maps["update_twin_counters"].aliased_param_pairs.contains(&(
            "counter_a".to_string(),
            "counter_b".to_string()
        )));
    }

    #[test]
    fn test_read_only_multilevel_is_shared_borrow() {
        let tu_ast = TranslationUnitAST {
            functions: vec![FunctionAST {
                name: "peek".to_string(),
                return_type: "int".to_string(),
                parameters: vec![ParameterAST {
                    name: "a".to_string(),
                    c_type: "int".to_string(),
                    pointer_depth: 2,
                    is_union: false,
                }],
                body_code: "return **a;".to_string(),
                call_sites: vec![],
            }],
            unions: vec![],
            structs: vec![],
        };

        let mut model = SemanticModel::new("peek".to_string());
        model.pointer_depth_classifications.insert("a".to_string(), PointerDepthClassification::MultiLevel { depth: 2 });
        model.memory_patterns.insert("a".to_string(), MemoryPattern::Read);

        let mut models = HashMap::new();
        models.insert("peek".to_string(), model);

        let ownership_maps = infer_ownership_types(&tu_ast, &models, &InterproceduralSummaries::new());
        assert!(matches!(
            ownership_maps["peek"].variable_ownership["a"],
            OwnershipKind::SharedBorrow
        ));
        assert_eq!(
            ownership_maps["peek"].param_lvalue_status["a"],
            LValueStoreStatus::ReadOnly
        );
    }
}
