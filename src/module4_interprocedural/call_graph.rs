use std::collections::HashMap;
use petgraph::graph::{Graph, NodeIndex};
use crate::module2_ast::TranslationUnitAST;
use crate::data_stores::interprocedural_summaries::CallSite;

pub struct CallGraphResult {
    pub graph: Graph<String, CallSite>,
    pub node_indices: HashMap<String, NodeIndex>,
}

/// 0.4.1 Call Graph Construction
/// Builds a whole-program call graph using petgraph, with functions as nodes and call sites as edges.
pub fn build_call_graph(tu_ast: &TranslationUnitAST) -> CallGraphResult {
    let mut graph = Graph::<String, CallSite>::new();
    let mut node_indices = HashMap::new();

    // 1. Add all function nodes
    for func in &tu_ast.functions {
        let node_idx = graph.add_node(func.name.clone());
        node_indices.insert(func.name.clone(), node_idx);
    }

    // 2. Add edges for each call site
    for func in &tu_ast.functions {
        if let Some(&caller_idx) = node_indices.get(&func.name) {
            for call in &func.call_sites {
                if let Some(&callee_idx) = node_indices.get(&call.callee) {
                    graph.add_edge(
                        caller_idx,
                        callee_idx,
                        CallSite {
                            caller: func.name.clone(),
                            callee: call.callee.clone(),
                            line: call.line,
                        },
                    );
                }
            }
        }
    }

    CallGraphResult { graph, node_indices }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module2_ast::ast_parser::{CallSiteAST, FunctionAST};

    #[test]
    fn test_call_graph_construction() {
        let tu_ast = TranslationUnitAST {
            functions: vec![
                FunctionAST {
                    name: "main".to_string(),
                    return_type: "int".to_string(),
                    parameters: vec![],
                    body_code: "foo();".to_string(),
                    call_sites: vec![CallSiteAST {
                        callee: "foo".to_string(),
                        arguments: vec![],
                        line: 10,
                    }],
                },
                FunctionAST {
                    name: "foo".to_string(),
                    return_type: "void".to_string(),
                    parameters: vec![],
                    body_code: "".to_string(),
                    call_sites: vec![],
                },
            ],
            unions: vec![],
            structs: vec![],
        };

        let result = build_call_graph(&tu_ast);
        assert_eq!(result.graph.node_count(), 2);
        assert_eq!(result.graph.edge_count(), 1);
    }
}
