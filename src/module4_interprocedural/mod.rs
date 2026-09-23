pub mod call_graph;
pub mod alias_analysis;
pub mod ownership_summary;
pub mod union_tracking;
pub mod data_dep_detector;

pub use call_graph::{build_call_graph, CallGraphResult};
pub use alias_analysis::analyze_interprocedural_aliasing;
pub use ownership_summary::generate_ownership_transfer_summaries;
pub use union_tracking::track_cross_function_union_variants;
pub use data_dep_detector::detect_data_dependent_ownership;

use std::collections::HashMap;
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::semantic_models::SemanticModel;
use crate::data_stores::interprocedural_summaries::InterproceduralSummaries;

/// Primary entry point for Module 4: Performs Interprocedural Analysis and populates D2 Interprocedural Summaries.
pub fn run_interprocedural_analysis(
    tu_ast: &TranslationUnitAST,
    semantic_models: &HashMap<String, SemanticModel>,
) -> InterproceduralSummaries {
    let mut summaries = InterproceduralSummaries::new();

    // 0.4.1 Call Graph Construction
    let call_graph_res = build_call_graph(tu_ast);
    summaries.call_graph = call_graph_res.graph;

    // 0.4.2 Interprocedural Alias & Points-to Analysis (Idiom 1 & Idiom 4)
    summaries.alias_verdicts = analyze_interprocedural_aliasing(tu_ast);

    // 0.4.3 Ownership-Transfer Summaries for Multi-Level Pointers (Idiom 2 & Idiom 3)
    summaries.ownership_transfer_summaries = generate_ownership_transfer_summaries(tu_ast, semantic_models);

    // 0.4.4 Cross-Function Union Variant Tracking (Idiom 5)
    summaries.cross_function_union_resolutions = track_cross_function_union_variants(tu_ast, semantic_models);

    // 0.4.5 Data-Dependent Ownership Detector (Idiom 6)
    summaries.data_dependent_ownership_flags = detect_data_dependent_ownership(tu_ast, semantic_models);

    summaries
}
