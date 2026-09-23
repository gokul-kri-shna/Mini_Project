use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Output struct produced when a C input file passes validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedCSource {
    pub file_path: PathBuf,
    pub raw_content: String,
    pub line_count: usize,
    pub byte_size: usize,
}

/// Errors that can occur during input acceptance and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    FileNotFound(String),
    InvalidExtension(String),
    FileNotReadable(String),
    InvalidEncoding(String),
    EmptyFile(String),
    PreflightLexicalError(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::FileNotFound(msg) => write!(f, "File Not Found Error: {}", msg),
            ValidationError::InvalidExtension(msg) => write!(f, "Invalid Extension Error: {}", msg),
            ValidationError::FileNotReadable(msg) => write!(f, "File Read Error: {}", msg),
            ValidationError::InvalidEncoding(msg) => write!(f, "Invalid Encoding Error: {}", msg),
            ValidationError::EmptyFile(msg) => write!(f, "Empty File Error: {}", msg),
            ValidationError::PreflightLexicalError(msg) => write!(f, "Lexical Validation Error: {}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Primary entry point for Module 1: Accepts a file path and validates it for C transpilation.
pub fn validate_input(file_path: &Path) -> Result<ValidatedCSource, ValidationError> {
    // 1. Check file existence
    if !file_path.exists() {
        return Err(ValidationError::FileNotFound(format!(
            "Input file does not exist: '{:?}'",
            file_path
        )));
    }

    // 2. Validate file extension (.c or .h allowed)
    if let Some(ext) = file_path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if ext_str != "c" && ext_str != "h" {
            return Err(ValidationError::InvalidExtension(format!(
                "Expected '.c' or '.h' extension, but found '.{}'",
                ext_str
            )));
        }
    } else {
        return Err(ValidationError::InvalidExtension(
            "Input file has no extension; expected a '.c' or '.h' file.".to_string(),
        ));
    }

    // 3. Read raw bytes to check readability and binary/encoding issues
    let bytes = fs::read(file_path).map_err(|err| {
        ValidationError::FileNotReadable(format!("Failed to read file '{:?}': {}", file_path, err))
    })?;

    // Quick binary content check (null bytes before UTF-8 decoding)
    if bytes.contains(&0) {
        return Err(ValidationError::InvalidEncoding(
            "Input file contains null bytes (binary file detected).".to_string(),
        ));
    }

    // 4. Validate UTF-8 encoding
    let content = String::from_utf8(bytes).map_err(|_| {
        ValidationError::InvalidEncoding(
            "Input file is not valid UTF-8 text.".to_string(),
        )
    })?;

    // 5. Validate non-empty content
    if content.trim().is_empty() {
        return Err(ValidationError::EmptyFile(format!(
            "Input file '{:?}' is empty or contains only whitespace.",
            file_path
        )));
    }

    // 6. Pre-flight lexical checks (balanced braces, parentheses, quotes)
    perform_preflight_lexical_checks(&content)?;

    let line_count = content.lines().count();
    let byte_size = content.len();

    Ok(ValidatedCSource {
        file_path: file_path.to_path_buf(),
        raw_content: content,
        line_count,
        byte_size,
    })
}

/// Lightweight lexical check to catch syntax sanity issues before AST parsing.
fn perform_preflight_lexical_checks(content: &str) -> Result<(), ValidationError> {
    let mut brace_depth = 0i32;
    let mut paren_depth = 0i32;
    let mut in_string = false;
    let mut in_char = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escaped = false;

    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        let next = if i + 1 < len { Some(chars[i + 1]) } else { None };

        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }

        if in_block_comment {
            if c == '*' && next == Some('/') {
                in_block_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if in_char {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '\'' {
                in_char = false;
            }
            i += 1;
            continue;
        }

        // Start comments
        if c == '/' && next == Some('/') {
            in_line_comment = true;
            i += 2;
            continue;
        }
        if c == '/' && next == Some('*') {
            in_block_comment = true;
            i += 2;
            continue;
        }

        // Literals
        if c == '"' {
            in_string = true;
            i += 1;
            continue;
        }
        if c == '\'' {
            in_char = true;
            i += 1;
            continue;
        }

        // Delimiters
        match c {
            '{' => brace_depth += 1,
            '}' => {
                brace_depth -= 1;
                if brace_depth < 0 {
                    return Err(ValidationError::PreflightLexicalError(
                        "Unmatched closing brace '}' found.".to_string(),
                    ));
                }
            }
            '(' => paren_depth += 1,
            ')' => {
                paren_depth -= 1;
                if paren_depth < 0 {
                    return Err(ValidationError::PreflightLexicalError(
                        "Unmatched closing parenthesis ')' found.".to_string(),
                    ));
                }
            }
            _ => {}
        }

        i += 1;
    }

    if in_string {
        return Err(ValidationError::PreflightLexicalError(
            "Unterminated string literal detected.".to_string(),
        ));
    }
    if in_block_comment {
        return Err(ValidationError::PreflightLexicalError(
            "Unterminated block comment detected.".to_string(),
        ));
    }
    if brace_depth > 0 {
        return Err(ValidationError::PreflightLexicalError(
            format!("Unclosed curly brace '{{' ({} missing '}}').", brace_depth),
        ));
    }
    if paren_depth > 0 {
        return Err(ValidationError::PreflightLexicalError(
            format!("Unclosed parenthesis '(' ({} missing ')').", paren_depth),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_valid_c_file() {
        let temp_path = Path::new("temp_valid.c");
        let mut file = fs::File::create(temp_path).unwrap();
        writeln!(
            file,
            "// Simple C Program\n#include <stdio.h>\nint main() {{ printf(\"Hello, World!\"); return 0; }}"
        )
        .unwrap();

        let result = validate_input(temp_path);
        let _ = fs::remove_file(temp_path);

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.line_count, 3);
        assert!(validated.byte_size > 0);
    }

    #[test]
    fn test_non_existent_file() {
        let path = Path::new("does_not_exist.c");
        let result = validate_input(path);
        assert!(matches!(result, Err(ValidationError::FileNotFound(_))));
    }

    #[test]
    fn test_invalid_extension() {
        let temp_path = Path::new("temp_file.txt");
        let mut file = fs::File::create(temp_path).unwrap();
        writeln!(file, "some text").unwrap();

        let result = validate_input(temp_path);
        let _ = fs::remove_file(temp_path);

        assert!(matches!(result, Err(ValidationError::InvalidExtension(_))));
    }

    #[test]
    fn test_empty_file() {
        let temp_path = Path::new("temp_empty.c");
        let _file = fs::File::create(temp_path).unwrap();

        let result = validate_input(temp_path);
        let _ = fs::remove_file(temp_path);

        assert!(matches!(result, Err(ValidationError::EmptyFile(_))));
    }

    #[test]
    fn test_binary_file() {
        let temp_path = Path::new("temp_binary.c");
        let mut file = fs::File::create(temp_path).unwrap();
        file.write_all(&[0x7f, 0x45, 0x4c, 0x46, 0x00, 0x01]).unwrap();

        let result = validate_input(temp_path);
        let _ = fs::remove_file(temp_path);

        assert!(matches!(result, Err(ValidationError::InvalidEncoding(_))));
    }

    #[test]
    fn test_unmatched_brace() {
        let temp_path = Path::new("temp_unmatched.c");
        let mut file = fs::File::create(temp_path).unwrap();
        writeln!(file, "int main() {{ return 0;").unwrap();

        let result = validate_input(temp_path);
        let _ = fs::remove_file(temp_path);

        assert!(matches!(result, Err(ValidationError::PreflightLexicalError(_))));
    }

    #[test]
    fn test_unterminated_string() {
        let temp_path = Path::new("temp_string.c");
        let mut file = fs::File::create(temp_path).unwrap();
        writeln!(file, "char *s = \"hello;").unwrap();

        let result = validate_input(temp_path);
        let _ = fs::remove_file(temp_path);

        assert!(matches!(result, Err(ValidationError::PreflightLexicalError(_))));
    }
}
