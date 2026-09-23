use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum MemoryPattern {
    Read,
    Write,
    Allocate,
    Free,
}

#[derive(Debug, Clone)]
pub enum PointerDepthClassification {
    SingleLevel,
    MultiLevel { depth: usize },
}

#[derive(Debug, Clone)]
pub struct UsePoint {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct UnionTransform {
    pub union_name: String,
    pub enum_name: String,
    pub local_variant_resolved: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UnresolvedUnionCase {
    pub union_name: String,
    pub function_name: String,
}

/// D1 Data Store: Per-Function Semantic Model
#[derive(Debug, Clone)]
pub struct SemanticModel {
    pub function_name: String,
    pub symbol_table: HashMap<String, String>,
    pub def_use_chains: HashMap<String, Vec<UsePoint>>,
    pub memory_patterns: HashMap<String, MemoryPattern>,
    pub pointer_depth_classifications: HashMap<String, PointerDepthClassification>,
    pub local_union_transforms: Vec<UnionTransform>,
    pub unresolved_union_cases: Vec<UnresolvedUnionCase>,
}

impl SemanticModel {
    pub fn new(function_name: String) -> Self {
        Self {
            function_name,
            symbol_table: HashMap::new(),
            def_use_chains: HashMap::new(),
            memory_patterns: HashMap::new(),
            pointer_depth_classifications: HashMap::new(),
            local_union_transforms: Vec::new(),
            unresolved_union_cases: Vec::new(),
        }
    }
}
