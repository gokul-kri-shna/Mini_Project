use std::collections::HashMap;
use std::fmt;
use std::process::Command;
use crate::module1_validation::ValidatedCSource;

#[derive(Debug, Clone)]
pub struct ExpandedCSource {
    pub raw_input: ValidatedCSource,
    pub expanded_content: String,
    pub used_external_preprocessor: bool,
}

#[derive(Debug, Clone)]
pub enum PreprocessorError {
    ExpansionFailed(String),
}

impl fmt::Display for PreprocessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreprocessorError::ExpansionFailed(msg) => write!(f, "Preprocessor Expansion Error: {}", msg),
        }
    }
}

impl std::error::Error for PreprocessorError {}

/// Main entry point for preprocessor expansion.
pub fn preprocess_c_code(input: &ValidatedCSource) -> Result<ExpandedCSource, PreprocessorError> {
    // Attempt external gcc -E preprocessing first
    if let Ok(expanded) = try_gcc_preprocessor(input) {
        return Ok(ExpandedCSource {
            raw_input: input.clone(),
            expanded_content: expanded,
            used_external_preprocessor: true,
        });
    }

    // Fallback to internal custom preprocessor for macro resolution and header normalization
    let expanded = fallback_internal_preprocessor(&input.raw_content)?;
    Ok(ExpandedCSource {
        raw_input: input.clone(),
        expanded_content: expanded,
        used_external_preprocessor: false,
    })
}

fn try_gcc_preprocessor(input: &ValidatedCSource) -> Result<String, PreprocessorError> {
    let output = Command::new("gcc")
        .arg("-E")
        .arg("-P")
        .arg(&input.file_path)
        .output()
        .map_err(|e| PreprocessorError::ExpansionFailed(e.to_string()))?;

    if output.status.success() {
        let content = String::from_utf8_lossy(&output.stdout).to_string();
        if !content.trim().is_empty() {
            return Ok(content);
        }
    }

    Err(PreprocessorError::ExpansionFailed(
        "GCC preprocessor failed or produced empty output.".to_string(),
    ))
}

/// Lightweight internal preprocessor to resolve #define, #ifdef/#ifndef, and strip system headers
fn fallback_internal_preprocessor(c_code: &str) -> Result<String, PreprocessorError> {
    let mut macros: HashMap<String, String> = HashMap::new();
    let mut processed_lines = Vec::new();
    let mut skip_stack = Vec::new();

    for line in c_code.lines() {
        let trimmed = line.trim();

        // Handle conditional compilation directives
        if trimmed.starts_with("#ifdef") {
            let macro_name = trimmed.trim_start_matches("#ifdef").trim();
            let is_defined = macros.contains_key(macro_name);
            skip_stack.push(!is_defined);
            continue;
        } else if trimmed.starts_with("#ifndef") {
            let macro_name = trimmed.trim_start_matches("#ifndef").trim();
            let is_defined = macros.contains_key(macro_name);
            skip_stack.push(is_defined);
            continue;
        } else if trimmed.starts_with("#else") {
            if let Some(top) = skip_stack.pop() {
                skip_stack.push(!top);
            }
            continue;
        } else if trimmed.starts_with("#endif") {
            skip_stack.pop();
            continue;
        }

        // If currently in a skipped conditional branch, skip this line
        if skip_stack.iter().any(|&skip| skip) {
            continue;
        }

        // Handle #define directives
        if trimmed.starts_with("#define") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[1].to_string();
                let val = if parts.len() >= 3 {
                    parts[2..].join(" ")
                } else {
                    "1".to_string()
                };
                macros.insert(name, val);
            }
            continue;
        }

        // Ignore standard system headers (#include <...>), preserve local comments/code
        if trimmed.starts_with("#include") {
            continue;
        }

        // Substitute simple macros in the line
        let mut expanded_line = line.to_string();
        for (name, val) in &macros {
            expanded_line = expanded_line.replace(name, val);
        }

        processed_lines.push(expanded_line);
    }

    Ok(processed_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_macro_expansion() {
        let code = r#"
#define BUFFER_SIZE 1024
#define DEBUG_MODE
#ifdef DEBUG_MODE
int size = BUFFER_SIZE;
#else
int size = 0;
#endif
"#;
        let expanded = fallback_internal_preprocessor(code).unwrap();
        assert!(expanded.contains("int size = 1024;"));
        assert!(!expanded.contains("int size = 0;"));
    }
}
