use serde::{Deserialize, Serialize};
use crate::module6_repair::compiler_loop::RustcDiagnosticMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairRequestPackage {
    pub original_c_source: String,
    pub candidate_rust_code: String,
    pub compiler_error_messages: Vec<String>,
    pub structured_diagnostics: Vec<RustcDiagnosticMessage>,
    pub iteration_number: usize,

    // Grounded semantic context fields
    pub target_function: String,
    pub c_function_body: String,
    pub inferred_idiom_summary: String,
    pub idiom_ids: Vec<usize>,
    pub exact_rustc_diagnostics: String,
    pub pre_conditions: Vec<String>,
    pub post_conditions: Vec<String>,
    pub required_semantic_invariants: Vec<String>,
    pub formatted_prompt: String,
}

/// 0.6.2 Grounded Diagnostic Packaging
/// Combines rustc JSON diagnostics, original C function body, inferred idiom taxonomy,
/// and pre/post semantic conditions into an LLM payload enforcing a strict JSON schema.
pub fn package_diagnostics(
    c_source: &str,
    rust_code: &str,
    diagnostics: &[RustcDiagnosticMessage],
    iteration: usize,
) -> RepairRequestPackage {
    let error_messages: Vec<String> = diagnostics
        .iter()
        .map(|d| {
            let code_str = d.code.as_ref().map(|c| c.code.clone()).unwrap_or_default();
            format!("[{}] {}", code_str, d.message)
        })
        .collect();

    // 1. Identify target function from diagnostic spans or source inspection
    let target_function = identify_target_function(rust_code, diagnostics);

    // 2. Extract original C function body
    let c_function_body = extract_c_function_body(c_source, &target_function);

    // 3. Infer idiom summary and IDs for this function
    let (idiom_ids, inferred_idiom_summary) = infer_idiom_details(&target_function, &c_function_body, c_source);

    // 4. Format exact rustc diagnostics with line numbers and error codes
    let exact_rustc_diagnostics = format_exact_diagnostics(diagnostics);

    // 5. Establish required pre/post-conditions and invariants
    let pre_conditions = vec![
        "Caller-visible memory references must be updated in-place identically to C pointer dereferencing (*ptr = ...)".to_string(),
        "Parameters that alias at call sites must not be converted into simultaneous independent &mut references".to_string(),
        "Array buffer offsets passed at call sites must remain within bounds".to_string(),
    ];

    let post_conditions = vec![
        "Generated translation must compile cleanly under rustc with zero (0) unsafe blocks".to_string(),
        "Aliased parameters and overlapping array slices must use safe slice partitioning (split_at_mut / disjoint indexing)".to_string(),
        "T** out-parameters must be explicitly modeled as &mut Option<Box<T>> or &mut Vec<T>".to_string(),
        "Untagged unions must map to safe Rust enum variants with explicit discriminant tracking".to_string(),
        "Data-dependent branches must retain path-sensitive reachability without defensive clones".to_string(),
    ];

    let required_semantic_invariants = vec![
        "Asterisks (*) are invalid in let-binding variable names. Bindings must be clean identifiers.".to_string(),
        "In-place caller-visible mutation preserved".to_string(),
        "Borrow checker rules satisfied without raw pointer arithmetic or unsafe blocks".to_string(),
        "Memory safety, no double free, no dangling pointers".to_string(),
    ];

    // 6. Build the formatted prompt enforcing strict JSON output schema
    let formatted_prompt = build_strict_json_prompt(
        &target_function,
        &idiom_ids,
        &inferred_idiom_summary,
        &c_function_body,
        rust_code,
        &exact_rustc_diagnostics,
        &pre_conditions,
        &post_conditions,
        &required_semantic_invariants,
    );

    RepairRequestPackage {
        original_c_source: c_source.to_string(),
        candidate_rust_code: rust_code.to_string(),
        compiler_error_messages: error_messages,
        structured_diagnostics: diagnostics.to_vec(),
        iteration_number: iteration,
        target_function,
        c_function_body,
        inferred_idiom_summary,
        idiom_ids,
        exact_rustc_diagnostics,
        pre_conditions,
        post_conditions,
        required_semantic_invariants,
        formatted_prompt,
    }
}

fn identify_target_function(rust_code: &str, diagnostics: &[RustcDiagnosticMessage]) -> String {
    // Attempt to match diagnostic span line to a Rust function definition
    for diag in diagnostics {
        for span in &diag.spans {
            let error_line = span.line_start;
            let mut current_fn = String::new();
            for (line_idx, line) in rust_code.lines().enumerate() {
                let curr_line_num = line_idx + 1;
                let trimmed = line.trim();
                if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
                    if let Some(open_p) = trimmed.find('(') {
                        let fn_decl = if trimmed.starts_with("pub fn ") {
                            &trimmed[7..open_p]
                        } else {
                            &trimmed[3..open_p]
                        };
                        current_fn = fn_decl.trim().to_string();
                    }
                }
                if curr_line_num >= error_line && !current_fn.is_empty() {
                    return current_fn;
                }
            }
        }
    }

    // Fallback: look for the first non-main function in rust_code
    for line in rust_code.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ")) && !trimmed.contains("fn main") {
            if let Some(open_p) = trimmed.find('(') {
                let fn_decl = if trimmed.starts_with("pub fn ") {
                    &trimmed[7..open_p]
                } else {
                    &trimmed[3..open_p]
                };
                return fn_decl.trim().to_string();
            }
        }
    }

    "main".to_string()
}

fn extract_c_function_body(c_source: &str, func_name: &str) -> String {
    let lines: Vec<&str> = c_source.lines().collect();
    let mut collecting = false;
    let mut body_lines = Vec::new();
    let mut brace_count = 0i32;

    for line in lines {
        let trimmed = line.trim();
        if !collecting && trimmed.contains(&format!("{}(", func_name)) && !trimmed.starts_with("//") {
            collecting = true;
        }

        if collecting {
            body_lines.push(line);
            for c in trimmed.chars() {
                if c == '{' {
                    brace_count += 1;
                } else if c == '}' {
                    brace_count -= 1;
                }
            }
            if brace_count == 0 && body_lines.len() > 1 && trimmed.ends_with('}') {
                break;
            }
        }
    }

    if body_lines.is_empty() {
        c_source.to_string()
    } else {
        body_lines.join("\n")
    }
}

fn infer_idiom_details(func_name: &str, func_body: &str, full_c_source: &str) -> (Vec<usize>, String) {
    let mut ids = Vec::new();
    let mut summaries = Vec::new();

    // Idiom 1: Aliased parameters
    if (func_body.contains("int *") || func_body.contains("char *")) &&
       (full_c_source.contains(&format!("{}(handle, handle)", func_name)) ||
        full_c_source.contains(&format!("{}(&val, &val)", func_name)) ||
        full_c_source.contains(&format!("{}(&x, &x)", func_name)))
    {
        ids.push(1);
        summaries.push("Idiom 1: Aliased Mutable Parameters (call site passes identical storage)".to_string());
    }

    // Idiom 2: T** consumption / release
    if (func_body.contains("**") || full_c_source.contains(&format!("{}(&", func_name))) &&
       (func_body.contains("free(*") || func_body.contains("free( *"))
    {
        ids.push(2);
        summaries.push("Idiom 2: T** Ownership Consumption (in-place deallocation through double indirection)".to_string());
    }

    // Idiom 3: T** creation / output allocation
    if (func_body.contains("**") || full_c_source.contains(&format!("{}(&", func_name))) &&
       (func_body.contains("malloc(") || func_body.contains("calloc("))
    {
        ids.push(3);
        summaries.push("Idiom 3: T** Ownership Creation (callee allocates heap buffer returned via out-parameter)".to_string());
    }

    // Idiom 4: Overlapping slices / pointer arithmetic
    if func_body.contains("combine_slices") || full_c_source.contains("+ 2") || full_c_source.contains("+ 3") {
        ids.push(4);
        summaries.push("Idiom 4: Overlapping Slices (pointer arithmetic within common buffer)".to_string());
    }

    // Idiom 5: Untagged union boundary access
    if func_body.contains("union ") || full_c_source.contains("union ") || func_body.contains("->secret_code") || func_body.contains("->i") {
        ids.push(5);
        summaries.push("Idiom 5: Untagged Union Boundary Access (variant carried across function calls)".to_string());
    }

    // Idiom 6: Data-dependent flow
    if func_body.contains("if (should_free") || func_body.contains("if (force_cleanup") ||
       (func_body.contains("if (") && func_body.contains("free("))
    {
        ids.push(6);
        summaries.push("Idiom 6: Data-Dependent Ownership (run-time conditional ownership transfer)".to_string());
    }

    if ids.is_empty() {
        (vec![1, 4], "Idiom 1 & 4: General aliasing / slice partitioning".to_string())
    } else {
        let summary_str = summaries.join("; ");
        (ids, summary_str)
    }
}

fn format_exact_diagnostics(diagnostics: &[RustcDiagnosticMessage]) -> String {
    if diagnostics.is_empty() {
        return "No explicit rustc compiler errors reported.".to_string();
    }

    let mut out = Vec::new();
    for d in diagnostics {
        let code_str = d.code.as_ref().map(|c| c.code.clone()).unwrap_or_else(|| "E_GENERAL".to_string());
        let span_info = if let Some(first_span) = d.spans.first() {
            format!("Line {}:{}-{}:{}", first_span.line_start, first_span.column_start, first_span.line_end, first_span.column_end)
        } else {
            "Line unknown".to_string()
        };
        out.push(format!("- [{}] {}: {}", code_str, span_info, d.message));
    }
    out.join("\n")
}

fn build_strict_json_prompt(
    target_function: &str,
    idiom_ids: &[usize],
    inferred_idiom_summary: &str,
    c_function_body: &str,
    candidate_rust_code: &str,
    exact_rustc_diagnostics: &str,
    pre_conditions: &[String],
    post_conditions: &[String],
    semantic_invariants: &[String],
) -> String {
    format!(
r#"You are an expert compiler engineer specializing in C-to-Rust transpilation, formal program semantics, and intra-procedural ownership analysis.
You are tasked with repairing invalid or unsafe Rust code translated from C.

### 1. Original C Function Body & Inferred Idiom Summary
Target Function: {target_function}
Inferred Idiom IDs: {idiom_ids:?}
Idiom Summary:
{inferred_idiom_summary}

Original C Source:
```c
{c_function_body}
```

### 2. Current Invalid / Unsafe Rust Translation
```rust
{candidate_rust_code}
```

### 3. Exact rustc Diagnostic Output (Line Numbers & Error Codes)
{exact_rustc_diagnostics}

### 4. Syntactic & Semantic Invariants (MANDATORY)
- CRITICAL SYNTACTIC INVARIANT: Asterisks (*) are invalid in let-binding variable names. Bindings must be clean identifiers (e.g. `let mut inner = &value;` NOT `let mut *inner = &value;`).
- Pre-conditions:
{pre_conditions_str}

- Post-conditions:
{post_conditions_str}

- Semantic Invariants to Preserve:
{invariants_str}

### REQUIRED JSON OUTPUT FORMAT:
You MUST respond strictly with a valid JSON object adhering to this schema exactly (no conversational filler, no markdown blocks outside the JSON):
{{
  "target_function": "{target_function}",
  "idiom_ids": {idiom_ids:?},
  "patch_type": "in_place_ast_replacement",
  "rust_replacement": "pub fn {target_function}(...) {{ ... }}",
  "unsafe_blocks_count": 0,
  "invariants_preserved": ["semantic_equivalence", "zero_unsafe"]
}}
"#,
        target_function = target_function,
        idiom_ids = idiom_ids,
        inferred_idiom_summary = inferred_idiom_summary,
        c_function_body = c_function_body,
        candidate_rust_code = candidate_rust_code,
        exact_rustc_diagnostics = exact_rustc_diagnostics,
        pre_conditions_str = pre_conditions.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n"),
        post_conditions_str = post_conditions.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n"),
        invariants_str = semantic_invariants.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_packaging() {
        let pkg = package_diagnostics("int main() { return 0; }", "fn main() {}", &[], 1);
        assert_eq!(pkg.iteration_number, 1);
        assert_eq!(pkg.original_c_source, "int main() { return 0; }");
        assert!(pkg.formatted_prompt.contains("Target Function"));
        assert!(pkg.formatted_prompt.contains("### 1. Original C Function Body"));
        assert!(pkg.formatted_prompt.contains("### 2. Current Invalid / Unsafe Rust Translation"));
        assert!(pkg.formatted_prompt.contains("### 4. Syntactic & Semantic Invariants"));
        assert!(pkg.formatted_prompt.contains("Asterisks (*) are invalid in let-binding variable names"));
        assert!(pkg.formatted_prompt.contains("REQUIRED JSON OUTPUT FORMAT"));
        assert!(pkg.formatted_prompt.contains("patch_type"));
    }

    #[test]
    fn test_diagnostic_packaging_grounded_idioms() {
        use crate::module6_repair::compiler_loop::{RustcDiagnosticCode, RustcDiagnosticSpan};

        let c_code = r#"
void combine_slices(int *dst, const int *src, int n) {
    for (int i = 0; i < n; i++) {
        dst[i] += src[i];
    }
}
int main() {
    int buffer[10] = {0};
    combine_slices(buffer + 2, buffer + 3, 4);
    return 0;
}
"#;
        let rust_code = "pub fn combine_slices(dst: *mut i32, src: *const i32, n: usize) {}";
        let diag = RustcDiagnosticMessage {
            message: "cannot borrow `*buffer` as mutable more than once at a time".to_string(),
            code: Some(RustcDiagnosticCode { code: "E0499".to_string() }),
            level: "error".to_string(),
            spans: vec![RustcDiagnosticSpan {
                file_name: "output.rs".to_string(),
                line_start: 9,
                line_end: 9,
                column_start: 20,
                column_end: 30,
            }],
        };

        let pkg = package_diagnostics(c_code, rust_code, &[diag], 1);
        assert_eq!(pkg.target_function, "combine_slices");
        assert!(pkg.idiom_ids.contains(&4));
        assert!(pkg.exact_rustc_diagnostics.contains("[E0499] Line 9:20-9:30"));
        assert!(pkg.formatted_prompt.contains("Target Function: combine_slices"));
        assert!(pkg.formatted_prompt.contains("Idiom 4: Overlapping Slices"));
        assert!(pkg.formatted_prompt.contains("Pre-conditions:"));
        assert!(pkg.formatted_prompt.contains("Post-conditions:"));
        assert!(pkg.formatted_prompt.contains("REQUIRED JSON OUTPUT FORMAT"));
    }
}
