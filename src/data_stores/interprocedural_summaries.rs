use std::collections::HashMap;
use petgraph::graph::Graph;

#[derive(Debug, Clone)]
pub struct CallSite {
    pub caller: String,
    pub callee: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct AliasingVerdict {
    pub arg1: String,
    pub arg2: String,
    pub aliases: bool,
}

#[derive(Debug, Clone)]
pub enum OwnershipTransferType {
    ConsumesOwnership, // Idiom 2 (callee frees caller's T**)
    CreatesOwnership,  // Idiom 3 (callee allocates T** out-param for caller)
    None,
}

#[derive(Debug, Clone)]
pub struct OwnershipTransferSummary {
    pub param_name: String,
    pub transfer_type: OwnershipTransferType,
}

#[derive(Debug, Clone)]
pub struct DataDepFlag {
    pub function_name: String,
    pub variable_name: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum OwnershipKind {
    UniqueHeapOwner,      // Box<T>
    MutableBorrow,        // &mut T
    SharedBorrow,         // &T
    HeapArray,            // Vec<T>
    ConservativeFallback, // Option<Box<T>>
}

/// D2 Data Store: Interprocedural Summaries
#[derive(Debug, Clone)]
pub struct InterproceduralSummaries {
    pub call_graph: Graph<String, CallSite>,
    pub alias_verdicts: HashMap<String, Vec<AliasingVerdict>>,
    pub ownership_transfer_summaries: HashMap<String, OwnershipTransferSummary>,
    pub cross_function_union_resolutions: HashMap<String, String>,
    pub data_dependent_ownership_flags: Vec<DataDepFlag>,
}

impl InterproceduralSummaries {
    pub fn new() -> Self {
        Self {
            call_graph: Graph::new(),
            alias_verdicts: HashMap::new(),
            ownership_transfer_summaries: HashMap::new(),
            cross_function_union_resolutions: HashMap::new(),
            data_dependent_ownership_flags: Vec::new(),
        }
    }
}
