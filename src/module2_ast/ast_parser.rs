use std::fmt;
use crate::module2_ast::preprocessor::ExpandedCSource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterAST {
    pub name: String,
    pub c_type: String,
    pub pointer_depth: usize, // 0 = T, 1 = T*, 2 = T**
    pub is_union: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSiteAST {
    pub callee: String,
    pub arguments: Vec<String>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionAST {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<ParameterAST>,
    pub body_code: String,
    pub call_sites: Vec<CallSiteAST>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionVariantAST {
    pub field_name: String,
    pub field_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionAST {
    pub name: String,
    pub variants: Vec<UnionVariantAST>,
    pub tag_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructAST {
    pub name: String,
    pub fields: Vec<(String, String)>,
}

/// Whole-Program AST Representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationUnitAST {
    pub functions: Vec<FunctionAST>,
    pub unions: Vec<UnionAST>,
    pub structs: Vec<StructAST>,
}

impl TranslationUnitAST {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            unions: Vec::new(),
            structs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ASTError {
    ParseFailed(String),
}

impl fmt::Display for ASTError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ASTError::ParseFailed(msg) => write!(f, "AST Parse Error: {}", msg),
        }
    }
}

impl std::error::Error for ASTError {}

/// Main entry point for AST Construction.
pub fn construct_ast(expanded: &ExpandedCSource) -> Result<TranslationUnitAST, ASTError> {
    let content = &expanded.expanded_content;
    let mut ast = TranslationUnitAST::new();

    // Parse C unions
    ast.unions = parse_c_unions(content);

    // Parse C structs
    ast.structs = parse_c_structs(content);

    // Parse C functions and call sites
    ast.functions = parse_c_functions(content);

    Ok(ast)
}

fn parse_c_unions(content: &str) -> Vec<UnionAST> {
    let mut unions = Vec::new();
    let mut in_union = false;
    let mut current_name = String::new();
    let mut current_variants = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("union ") && trimmed.contains('{') {
            in_union = true;
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                current_name = parts[1].trim_matches('{').to_string();
            }
            current_variants.clear();
            continue;
        }

        if in_union {
            if trimmed.starts_with('}') {
                in_union = false;
                unions.push(UnionAST {
                    name: current_name.clone(),
                    variants: current_variants.clone(),
                    tag_field: None, // Will be resolved during semantic analysis
                });
                continue;
            }

            // Parse field declaration inside union
            if trimmed.contains(';') {
                let parts: Vec<&str> = trimmed.trim_matches(';').split_whitespace().collect();
                if parts.len() >= 2 {
                    let field_type = parts[0].to_string();
                    let field_name = parts[1].trim_matches('*').to_string();
                    current_variants.push(UnionVariantAST {
                        field_name,
                        field_type,
                    });
                }
            }
        }
    }

    unions
}

fn parse_c_structs(content: &str) -> Vec<StructAST> {
    let mut structs = Vec::new();
    let mut in_struct = false;
    let mut current_name = String::new();
    let mut current_fields = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("struct ") && trimmed.contains('{') {
            in_struct = true;
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                current_name = parts[1].trim_matches('{').to_string();
            }
            current_fields.clear();
            continue;
        }

        if in_struct {
            if trimmed.starts_with('}') {
                in_struct = false;
                structs.push(StructAST {
                    name: current_name.clone(),
                    fields: current_fields.clone(),
                });
                continue;
            }

            if trimmed.contains(';') {
                let parts: Vec<&str> = trimmed.trim_matches(';').split_whitespace().collect();
                if parts.len() >= 2 {
                    let field_type = parts[0].to_string();
                    let field_name = parts[1].to_string();
                    current_fields.push((field_name, field_type));
                }
            }
        }
    }

    structs
}

fn parse_c_functions(content: &str) -> Vec<FunctionAST> {
    let mut functions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Detect function signature: e.g. "void swap(int **a, int **b) {"
        if line.contains('(') && line.contains(')') && (line.ends_with('{') || (i + 1 < lines.len() && lines[i + 1].trim() == "{")) {
            let sig_line = if line.ends_with('{') {
                line.trim_end_matches('{').trim()
            } else {
                line
            };

            if let Some(func) = parse_single_function_signature(sig_line, &lines, &mut i) {
                functions.push(func);
            }
        }
        i += 1;
    }

    functions
}

fn parse_single_function_signature(sig_line: &str, lines: &[&str], line_idx: &mut usize) -> Option<FunctionAST> {
    let open_paren = sig_line.find('(')?;
    let close_paren = sig_line.rfind(')')?;

    let head = sig_line[..open_paren].trim();
    let head_parts: Vec<&str> = head.split_whitespace().collect();
    if head_parts.len() < 2 {
        return None;
    }

    let return_type = head_parts[..head_parts.len() - 1].join(" ");
    let func_name = head_parts[head_parts.len() - 1].trim_matches('*').to_string();

    let params_str = &sig_line[open_paren + 1..close_paren].trim();
    let parameters = parse_parameters(params_str);

    // Collect body lines and find call sites inside function body
    let mut body_lines = Vec::new();
    let mut call_sites = Vec::new();
    let mut brace_count = 0i32;
    let mut started = false;

    let start_line = *line_idx;
    for (curr_line_num, &raw_l) in lines.iter().enumerate().skip(start_line) {
        let l = raw_l.trim();
        for c in l.chars() {
            if c == '{' {
                brace_count += 1;
                started = true;
            } else if c == '}' {
                brace_count -= 1;
            }
        }

        if started {
            body_lines.push(l.to_string());

            // Detect call sites (e.g. swap(&px, &py);)
            if l.contains('(') && l.contains(')') && l.ends_with(';') && !l.starts_with("if") && !l.starts_with("while") && !l.starts_with("for") && !l.starts_with("return") {
                if let Some(call) = parse_call_site(l, curr_line_num + 1) {
                    call_sites.push(call);
                }
            }

            if brace_count == 0 {
                *line_idx = curr_line_num;
                break;
            }
        }
    }

    Some(FunctionAST {
        name: func_name,
        return_type,
        parameters,
        body_code: body_lines.join("\n"),
        call_sites,
    })
}

fn parse_parameters(params_str: &str) -> Vec<ParameterAST> {
    let mut params = Vec::new();
    if params_str.is_empty() || params_str == "void" {
        return params;
    }

    for param_decl in params_str.split(',') {
        let trimmed = param_decl.trim();
        if trimmed.is_empty() {
            continue;
        }

        let pointer_depth = trimmed.chars().filter(|&c| c == '*').count();
        let is_union = trimmed.contains("union ");

        let clean_decl = trimmed.replace('*', " ");
        let parts: Vec<&str> = clean_decl.split_whitespace().collect();

        let (param_name, c_type) = if parts.len() >= 2 {
            let name = parts.last().unwrap().to_string();
            let base_type = parts[..parts.len() - 1].join(" ");
            (name, base_type)
        } else {
            ("param".to_string(), trimmed.to_string())
        };

        params.push(ParameterAST {
            name: param_name,
            c_type,
            pointer_depth,
            is_union,
        });
    }

    params
}

fn parse_call_site(line: &str, line_num: usize) -> Option<CallSiteAST> {
    let open_p = line.find('(')?;
    let close_p = line.rfind(')')?;

    let left = line[..open_p].trim();
    let callee_parts: Vec<&str> = left.split_whitespace().collect();
    let callee = callee_parts.last()?.trim_matches(|c| c == '=' || c == '+' || c == '-').to_string();

    if callee.is_empty() || callee == "printf" || callee == "scanf" {
        return None;
    }

    let args_str = &line[open_p + 1..close_p];
    let arguments: Vec<String> = args_str.split(',').map(|s| s.trim().to_string()).collect();

    Some(CallSiteAST {
        callee,
        arguments,
        line: line_num,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_and_call_site_parsing() {
        let code = r#"
void swap(int **a, int **b) {
    int *temp = *a;
    *a = *b;
    *b = temp;
}

int main() {
    int x = 10;
    int y = 20;
    int *px = &x;
    int *py = &y;
    swap(&px, &py);
    return 0;
}
"#;
        let functions = parse_c_functions(code);
        assert_eq!(functions.len(), 2);

        let swap_fn = &functions[0];
        assert_eq!(swap_fn.name, "swap");
        assert_eq!(swap_fn.parameters.len(), 2);
        assert_eq!(swap_fn.parameters[0].pointer_depth, 2); // int **a

        let main_fn = &functions[1];
        assert_eq!(main_fn.name, "main");
        assert_eq!(main_fn.call_sites.len(), 1);
        assert_eq!(main_fn.call_sites[0].callee, "swap");
        assert_eq!(main_fn.call_sites[0].arguments, vec!["&px", "&py"]);
    }
}
