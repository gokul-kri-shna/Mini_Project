use std::env;
use serde_json::json;
use crate::module6_repair::diagnostic_packager::RepairRequestPackage;

#[derive(Debug, Clone)]
pub struct PatchProposal {
    pub patched_rust_code: String,
    pub explanation: String,
    pub source: String, // "Gemini-LLM-API" or "Rule-Based-Fallback"
}

/// 0.6.3 LLM API Client
/// Dispatches diagnostic packages to Google Gemini API (or rule-based fallback if API key is absent/offline).
pub fn query_llm_repair_agent(pkg: &RepairRequestPackage) -> Result<PatchProposal, String> {
    // 1. Check environment variable or read from local .env file
    let api_key = env::var("GEMINI_API_KEY")
        .ok()
        .or_else(|| read_key_from_dotenv());

    if let Some(key) = api_key {
        if !key.trim().is_empty() {
            println!("   📡 Sending diagnostic package to Gemini LLM Repair API...");
            match call_gemini_api(&key, pkg) {
                Ok(proposal) => return Ok(proposal),
                Err(err) => {
                    println!("   ⚠️ Gemini API call encountered issue: {}. Escalating to fallback repair engine.", err);
                }
            }
        }
    }

    println!("   ⚙️ Using rule-based repair agent fallback...");
    Ok(fallback_rule_based_repair(pkg))
}

fn read_key_from_dotenv() -> Option<String> {
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("GEMINI_API_KEY=") {
                let val = trimmed.trim_start_matches("GEMINI_API_KEY=").trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn call_gemini_api(api_key: &str, pkg: &RepairRequestPackage) -> Result<PatchProposal, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.6-flash:generateContent?key={}",
        api_key
    );

    let prompt = format!(
        "You are an expert Rust compiler repair agent for C-to-Rust transpilation.\n\
        The following Rust code was synthesized from C, but failed to compile with rustc.\n\n\
        === Original C Source ===\n{}\n\n\
        === Candidate Rust Code ===\n{}\n\n\
        === rustc Diagnostic Errors ===\n{}\n\n\
        Please provide the REPAIRED valid Rust code only enclosed in ```rust ... ``` blocks, preserving memory safety and ownership intent.",
        pkg.original_c_source,
        pkg.candidate_rust_code,
        pkg.compiler_error_messages.join("\n")
    );

    let payload = json!({
        "contents": [{
            "parts": [{ "text": prompt }]
        }]
    });

    let client = reqwest::blocking::Client::new();
    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .map_err(|e| format!("HTTP request error: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Gemini API HTTP Error Status: {}", res.status()));
    }

    let json_resp: serde_json::Value = res
        .json()
        .map_err(|e| format!("Failed to parse Gemini response JSON: {}", e))?;

    let text = json_resp["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if text.is_empty() {
        return Err("Gemini API returned empty text response.".to_string());
    }

    let patched_code = extract_rust_code_block(&text).unwrap_or(text.clone());

    Ok(PatchProposal {
        patched_rust_code: patched_code,
        explanation: "Patched via Gemini LLM API".to_string(),
        source: "Gemini-LLM-API".to_string(),
    })
}

fn fallback_rule_based_repair(pkg: &RepairRequestPackage) -> PatchProposal {
    let mut patched = pkg.candidate_rust_code.clone();

    // Clean syntax issues (e.g. invalid C header signatures embedded in Rust)
    let lines: Vec<&str> = patched.lines().collect();
    let mut clean_lines = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("void ") || trimmed.starts_with("int main()") {
            continue; // strip duplicate raw C headers
        }
        clean_lines.push(line);
    }

    patched = clean_lines.join("\n");

    PatchProposal {
        patched_rust_code: patched,
        explanation: "Applied syntax normalization and rule-based repair fallback.".to_string(),
        source: "Rule-Based-Fallback".to_string(),
    }
}

fn extract_rust_code_block(text: &str) -> Option<String> {
    if let Some(start) = text.find("```rust") {
        let after_start = &text[start + 7..];
        if let Some(end) = after_start.find("```") {
            return Some(after_start[..end].trim().to_string());
        }
    } else if let Some(start) = text.find("```") {
        let after_start = &text[start + 3..];
        if let Some(end) = after_start.find("```") {
            return Some(after_start[..end].trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_code_block() {
        let input = "Here is the code:\n```rust\nfn main() {}\n```\nHope this helps!";
        let extracted = extract_rust_code_block(input);
        assert_eq!(extracted, Some("fn main() {}".to_string()));
    }
}
