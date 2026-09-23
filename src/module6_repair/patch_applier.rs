use std::fs;
use crate::module6_repair::llm_client::PatchProposal;

#[derive(Debug, Clone)]
pub struct RepairLogEntry {
    pub iteration: usize,
    pub patch_source: String,
    pub explanation: String,
    pub compilation_successful: bool,
}

/// 0.6.4 Patch Application and Re-verification
/// Writes candidate patch to the target Rust file and logs execution status.
pub fn apply_patch_to_file(
    rs_file_path: &str,
    proposal: &PatchProposal,
    iteration: usize,
) -> Result<RepairLogEntry, String> {
    fs::write(rs_file_path, &proposal.patched_rust_code)
        .map_err(|e| format!("Failed to apply patch to file '{}': {}", rs_file_path, e))?;

    Ok(RepairLogEntry {
        iteration,
        patch_source: proposal.source.clone(),
        explanation: proposal.explanation.clone(),
        compilation_successful: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_patch() {
        let temp_file = "temp_patch.rs";
        let proposal = PatchProposal {
            patched_rust_code: "pub fn test() {}".to_string(),
            explanation: "test patch".to_string(),
            source: "unit_test".to_string(),
        };

        let res = apply_patch_to_file(temp_file, &proposal, 1);
        let content = fs::read_to_string(temp_file).unwrap();
        let _ = fs::remove_file(temp_file);

        assert!(res.is_ok());
        assert_eq!(content, "pub fn test() {}");
    }
}
