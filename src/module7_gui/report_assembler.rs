use std::collections::HashMap;
use crate::module1_validation::ValidatedCSource;
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::semantic_models::SemanticModel;
use crate::data_stores::interprocedural_summaries::InterproceduralSummaries;
use crate::module6_repair::RepairResult;

#[derive(Debug, Clone)]
pub struct IdiomStatus {
    pub idiom_number: usize,
    pub name: &'static str,
    pub detected: bool,
    pub resolution: String,
}

#[derive(Debug, Clone)]
pub struct TranslationReport {
    pub source_file_name: String,
    pub c_lines: usize,
    pub c_bytes: usize,
    pub functions_count: usize,
    pub call_graph_nodes: usize,
    pub call_graph_edges: usize,
    pub idioms_summary: Vec<IdiomStatus>,
    pub repair_iterations: usize,
    pub compilation_successful: bool,
    pub final_rust_code: String,
    pub formatted_markdown_report: String,
}

/// 0.7.1 Report Assembly
/// Collects decision logs, repair histories, idiom transformations, and metrics into a TranslationReport.
pub fn assemble_translation_report(
    validated: &ValidatedCSource,
    tu_ast: &TranslationUnitAST,
    _semantic_models: &HashMap<String, SemanticModel>,
    summaries: &InterproceduralSummaries,
    repair_res: &RepairResult,
) -> TranslationReport {
    let mut idioms = Vec::new();

    // Idiom 1: Aliased &mut parameters
    let idiom1_detected = summaries.alias_verdicts.values().any(|verdicts| verdicts.iter().any(|v| v.aliases));
    idioms.push(IdiomStatus {
        idiom_number: 1,
        name: "Aliased &mut parameters",
        detected: idiom1_detected,
        resolution: if idiom1_detected {
            "Interprocedural points-to alias analysis over call-site arguments".to_string()
        } else {
            "No call-site aliasing detected".to_string()
        },
    });

    // Idiom 2: T** consumes ownership
    let idiom2_detected = summaries.ownership_transfer_summaries.values().any(|s| matches!(s.transfer_type, crate::data_stores::interprocedural_summaries::OwnershipTransferType::ConsumesOwnership));
    idioms.push(IdiomStatus {
        idiom_number: 2,
        name: "Ownership consumed through T** argument",
        detected: idiom2_detected,
        resolution: if idiom2_detected {
            "Def-use link from callee's write-through pointer back to caller's variable".to_string()
        } else {
            "None detected".to_string()
        },
    });

    // Idiom 3: T** creates ownership
    let idiom3_detected = summaries.ownership_transfer_summaries.values().any(|s| matches!(s.transfer_type, crate::data_stores::interprocedural_summaries::OwnershipTransferType::CreatesOwnership));
    idioms.push(IdiomStatus {
        idiom_number: 3,
        name: "Ownership created through T** out-parameter",
        detected: idiom3_detected,
        resolution: if idiom3_detected {
            "Callee heap allocation linked back to caller's local variable".to_string()
        } else {
            "None detected".to_string()
        },
    });

    // Idiom 4: Overlapping pointer-arithmetic slices
    let idiom4_detected = summaries.alias_verdicts.values().any(|verdicts| verdicts.iter().any(|v| v.arg1.contains('+') || v.arg2.contains('+')));
    idioms.push(IdiomStatus {
        idiom_number: 4,
        name: "Overlapping pointer-arithmetic slices",
        detected: idiom4_detected,
        resolution: if idiom4_detected {
            "Call-site range offset overlap check".to_string()
        } else {
            "None detected".to_string()
        },
    });

    // Idiom 5: Union variant set in another function
    let idiom5_detected = !summaries.cross_function_union_resolutions.is_empty();
    idioms.push(IdiomStatus {
        idiom_number: 5,
        name: "Union's active variant carried across call boundary",
        detected: idiom5_detected,
        resolution: if idiom5_detected {
            "Interprocedural reaching-definitions over tag/variant state".to_string()
        } else {
            "None detected".to_string()
        },
    });

    // Idiom 6: Data-dependent ownership
    let idiom6_detected = !summaries.data_dependent_ownership_flags.is_empty();
    idioms.push(IdiomStatus {
        idiom_number: 6,
        name: "Data-dependent ownership (run-time)",
        detected: idiom6_detected,
        resolution: if idiom6_detected {
            "Routed to Option<Box<T>> fallback or LLM repair agent".to_string()
        } else {
            "None detected".to_string()
        },
    });

    let mut markdown = String::new();
    markdown.push_str(&format!("# Translation Report for {:?}\n\n", validated.file_path.file_name().unwrap_or_default()));
    markdown.push_str(&format!("- **Input C File**: {} bytes, {} lines\n", validated.byte_size, validated.line_count));
    markdown.push_str(&format!("- **Parsed Functions**: {}\n", tu_ast.functions.len()));
    markdown.push_str(&format!("- **Call Graph**: {} nodes, {} edges\n", summaries.call_graph.node_count(), summaries.call_graph.edge_count()));
    markdown.push_str(&format!("- **Final Compilation Success**: {}\n", repair_res.final_success));
    markdown.push_str(&format!("- **Repair Iterations Used**: {}\n\n", repair_res.iterations_used));

    markdown.push_str("## 6 C Idioms Taxonomy Summary\n\n");
    for idm in &idioms {
        let status_str = if idm.detected { "DETECTED" } else { "NOT DETECTED" };
        markdown.push_str(&format!("- **Idiom {} ({})**: [{}] — {}\n", idm.idiom_number, idm.name, status_str, idm.resolution));
    }

    markdown.push_str("\n## Repair History\n\n");
    if repair_res.repair_logs.is_empty() {
        markdown.push_str("No repair iterations required (clean translation on initial synthesis).\n");
    } else {
        for log in &repair_res.repair_logs {
            markdown.push_str(&format!("- Iteration {}: Source: {} | Note: {}\n", log.iteration, log.patch_source, log.explanation));
        }
    }

    TranslationReport {
        source_file_name: validated.file_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        c_lines: validated.line_count,
        c_bytes: validated.byte_size,
        functions_count: tu_ast.functions.len(),
        call_graph_nodes: summaries.call_graph.node_count(),
        call_graph_edges: summaries.call_graph.edge_count(),
        idioms_summary: idioms,
        repair_iterations: repair_res.iterations_used,
        compilation_successful: repair_res.final_success,
        final_rust_code: repair_res.final_rust_code.clone(),
        formatted_markdown_report: markdown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_report_assembly() {
        let val = ValidatedCSource {
            file_path: PathBuf::from("test.c"),
            raw_content: "int main() {}".to_string(),
            line_count: 1,
            byte_size: 13,
        };

        let tu_ast = TranslationUnitAST::new();
        let models = HashMap::new();
        let summaries = InterproceduralSummaries::new();
        let repair_res = RepairResult {
            final_success: true,
            iterations_used: 1,
            repair_logs: vec![],
            final_rust_code: "pub fn main() {}".to_string(),
        };

        let report = assemble_translation_report(&val, &tu_ast, &models, &summaries, &repair_res);
        assert_eq!(report.idioms_summary.len(), 6);
        assert!(report.compilation_successful);
    }
}
