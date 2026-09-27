use std::fs;
use crate::module6_repair::llm_client::PatchProposal;

#[derive(Debug, Clone)]
pub struct RepairLogEntry {
    pub iteration: usize,
    pub patch_source: String,
    pub explanation: String,
    pub compilation_successful: bool,
    pub target_function: Option<String>,
    pub idiom_ids: Vec<usize>,
    pub unsafe_count_before: usize,
    pub unsafe_count_after: usize,
    pub invariants_preserved: Vec<String>,
}

/// 0.6.4 Patch Application and Re-verification
/// Writes candidate patch to the target Rust file and logs execution status with idiom metadata.
pub fn apply_patch_to_file(
    rs_file_path: &str,
    proposal: &PatchProposal,
    iteration: usize,
) -> Result<RepairLogEntry, String> {
    let existing_code = fs::read_to_string(rs_file_path).unwrap_or_default();
    let existing_unsafe_count = existing_code.matches("unsafe").count();
    let proposed_unsafe_count = proposal.patched_rust_code.matches("unsafe").count();

    // 1. Validate that the patch does not introduce new unsafe blocks
    if proposed_unsafe_count > existing_unsafe_count {
        return Err(format!(
            "Patch rejected: proposed patch introduces {} new unsafe block(s) (existing: {}, proposed: {}).",
            proposed_unsafe_count - existing_unsafe_count,
            existing_unsafe_count,
            proposed_unsafe_count
        ));
    }

    // 2. Validate AST node/function replacement scoping
    if let Some(ref r) = proposal.repair_response {
        if r.patch_type == "in_place_ast_replacement" && !r.target_function.is_empty() {
            let fn_sig1 = format!("pub fn {}", r.target_function);
            let fn_sig2 = format!("fn {}", r.target_function);
            if !r.rust_replacement.contains(&fn_sig1) && !r.rust_replacement.contains(&fn_sig2) {
                return Err(format!(
                    "Patch rejected: patch_type is 'in_place_ast_replacement' but replacement does not contain target function '{}'.",
                    r.target_function
                ));
            }
        }
        if r.invariants_preserved.iter().any(|inv| inv == "zero_unsafe") && proposed_unsafe_count > 0 {
            return Err("Patch rejected: invariants_preserved specified 'zero_unsafe' but candidate contains unsafe blocks.".to_string());
        }
    }

    fs::write(rs_file_path, &proposal.patched_rust_code)
        .map_err(|e| format!("Failed to apply patch to file '{}': {}", rs_file_path, e))?;

    let (target_fn, idiom_ids, unsafe_before, unsafe_after, invariants) = if let Some(ref r) = proposal.repair_response {
        (
            Some(r.target_function.clone()),
            r.idiom_ids.clone(),
            r.unsafe_count_before,
            r.unsafe_blocks_count,
            r.invariants_preserved.clone(),
        )
    } else {
        (None, Vec::new(), 0, 0, Vec::new())
    };

    Ok(RepairLogEntry {
        iteration,
        patch_source: proposal.source.clone(),
        explanation: proposal.explanation.clone(),
        compilation_successful: false, // Will be updated after re-compilation in compiler loop
        target_function: target_fn,
        idiom_ids,
        unsafe_count_before: unsafe_before,
        unsafe_count_after: unsafe_after,
        invariants_preserved: invariants,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module6_repair::llm_client::LlmRepairResponse;

    #[test]
    fn test_apply_patch() {
        let temp_file = "temp_patch.rs";
        let proposal = PatchProposal {
            patched_rust_code: "pub fn test() {}".to_string(),
            explanation: "test patch".to_string(),
            source: "unit_test".to_string(),
            repair_response: Some(LlmRepairResponse {
                target_function: "test".to_string(),
                idiom_ids: vec![1, 4],
                patch_type: "in_place_ast_replacement".to_string(),
                rust_replacement: "pub fn test() {}".to_string(),
                unsafe_count_before: 1,
                unsafe_blocks_count: 0,
                invariants_preserved: vec!["safe partitioning".to_string()],
            }),
        };

        let res = apply_patch_to_file(temp_file, &proposal, 1);
        let content = fs::read_to_string(temp_file).unwrap();
        let _ = fs::remove_file(temp_file);

        assert!(res.is_ok());
        let log = res.unwrap();
        assert_eq!(content, "pub fn test() {}");
        assert_eq!(log.target_function, Some("test".to_string()));
        assert_eq!(log.idiom_ids, vec![1, 4]);
        assert_eq!(log.unsafe_count_before, 1);
        assert_eq!(log.unsafe_count_after, 0);
    }

    #[test]
    fn test_reject_unsafe_injection() {
        let temp_file = "temp_safe.rs";
        fs::write(temp_file, "pub fn safe_fn() {}").unwrap();

        let proposal = PatchProposal {
            patched_rust_code: "pub fn safe_fn() { unsafe { } }".to_string(),
            explanation: "unsafe injection".to_string(),
            source: "malicious_patch".to_string(),
            repair_response: Some(LlmRepairResponse {
                target_function: "safe_fn".to_string(),
                idiom_ids: vec![],
                patch_type: "in_place_ast_replacement".to_string(),
                rust_replacement: "pub fn safe_fn() { unsafe { } }".to_string(),
                unsafe_count_before: 0,
                unsafe_blocks_count: 1,
                invariants_preserved: vec![],
            }),
        };

        let res = apply_patch_to_file(temp_file, &proposal, 1);
        let _ = fs::remove_file(temp_file);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Patch rejected: proposed patch introduces 1 new unsafe block"));
    }
}
