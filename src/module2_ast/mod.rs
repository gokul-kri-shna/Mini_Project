pub mod preprocessor;
pub mod ast_parser;

pub use preprocessor::{preprocess_c_code, ExpandedCSource, PreprocessorError};
pub use ast_parser::{construct_ast, TranslationUnitAST, FunctionAST, ParameterAST, CallSiteAST, UnionAST, StructAST, ASTError};

use crate::module1_validation::ValidatedCSource;

#[derive(Debug, Clone)]
pub struct ProcessedModule2Output {
    pub expanded_source: ExpandedCSource,
    pub ast: TranslationUnitAST,
}

/// Primary entry point for Module 2: Executes Preprocessor Expansion and AST Construction.
pub fn preprocess_and_parse(validated: &ValidatedCSource) -> Result<ProcessedModule2Output, String> {
    let expanded = preprocess_c_code(validated)
        .map_err(|err| format!("Preprocessor Error: {}", err))?;

    let ast = construct_ast(&expanded)
        .map_err(|err| format!("AST Parse Error: {}", err))?;

    Ok(ProcessedModule2Output {
        expanded_source: expanded,
        ast,
    })
}
