use serde::{Deserialize, Serialize};
use crate::module6_repair::compiler_loop::RustcDiagnosticMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairRequestPackage {
    pub original_c_source: String,
    pub candidate_rust_code: String,
    pub compiler_error_messages: Vec<String>,
    pub structured_diagnostics: Vec<RustcDiagnosticMessage>,
    pub iteration_number: usize,
}

/// 0.6.2 Diagnostic Packaging
/// Combines rustc JSON diagnostics, original C source, and candidate Rust code into an LLM payload.
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

    RepairRequestPackage {
        original_c_source: c_source.to_string(),
        candidate_rust_code: rust_code.to_string(),
        compiler_error_messages: error_messages,
        structured_diagnostics: diagnostics.to_vec(),
        iteration_number: iteration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_packaging() {
        let pkg = package_diagnostics("int main() { return 0; }", "fn main() {}", &[], 1);
        assert_eq!(pkg.iteration_number, 1);
        assert_eq!(pkg.original_c_source, "int main() { return 0; }");
    }
}
