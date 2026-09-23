use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustcDiagnosticSpan {
    pub file_name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: usize,
    pub column_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustcDiagnosticCode {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustcDiagnosticMessage {
    pub message: String,
    pub code: Option<RustcDiagnosticCode>,
    pub level: String, // "error", "warning", etc.
    pub spans: Vec<RustcDiagnosticSpan>,
}

#[derive(Debug, Clone)]
pub struct CompilationResult {
    pub success: bool,
    pub raw_output: String,
    pub diagnostics: Vec<RustcDiagnosticMessage>,
    pub error_count: usize,
}

/// 0.6.1 Compile Code
/// Submits candidate Rust code to rustc with --error-format=json and parses diagnostic errors.
pub fn invoke_rustc_compiler(rs_file_path: &str) -> CompilationResult {
    let output = Command::new("rustc")
        .arg("--error-format=json")
        .arg("--crate-type=lib")
        .arg(rs_file_path)
        .output();

    match output {
        Ok(out) => {
            let stderr_str = String::from_utf8_lossy(&out.stderr).to_string();
            let mut diagnostics = Vec::new();
            let mut error_count = 0;

            for line in stderr_str.lines() {
                if let Ok(diag) = serde_json::from_str::<RustcDiagnosticMessage>(line) {
                    if diag.level == "error" {
                        error_count += 1;
                    }
                    diagnostics.push(diag);
                }
            }

            CompilationResult {
                success: out.status.success() && error_count == 0,
                raw_output: stderr_str,
                diagnostics,
                error_count,
            }
        }
        Err(err) => CompilationResult {
            success: false,
            raw_output: format!("Failed to execute rustc: {}", err),
            diagnostics: Vec::new(),
            error_count: 1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_rustc_invocation_valid_code() {
        let test_file = "temp_valid.rs";
        fs::write(test_file, "pub fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();

        let res = invoke_rustc_compiler(test_file);
        let _ = fs::remove_file(test_file);
        let _ = fs::remove_file("libtemp_valid.rlib"); // Cleanup artifact if created

        assert!(res.success);
        assert_eq!(res.error_count, 0);
    }
}
