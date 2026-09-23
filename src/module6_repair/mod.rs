pub mod compiler_loop;
pub mod diagnostic_packager;
pub mod llm_client;
pub mod patch_applier;

pub use compiler_loop::{invoke_rustc_compiler, CompilationResult};
pub use diagnostic_packager::{package_diagnostics, RepairRequestPackage};
pub use llm_client::{query_llm_repair_agent, PatchProposal};
pub use patch_applier::{apply_patch_to_file, RepairLogEntry};

#[derive(Debug, Clone)]
pub struct RepairResult {
    pub final_success: bool,
    pub iterations_used: usize,
    pub repair_logs: Vec<RepairLogEntry>,
    pub final_rust_code: String,
}

/// Primary entry point for Module 6: Executes the Compiler Validation & LLM-Guided Repair Cycle.
pub fn run_repair_loop(
    c_source: &str,
    candidate_rust_path: &str,
    max_iterations: usize,
) -> Result<RepairResult, String> {
    let mut logs = Vec::new();
    let mut current_rust = std::fs::read_to_string(candidate_rust_path)
        .map_err(|e| format!("Failed to read candidate Rust file '{}': {}", candidate_rust_path, e))?;

    for iteration in 1..=max_iterations {
        println!("   🔍 Compiler Loop Iteration {}/{}...", iteration, max_iterations);
        
        let comp_res = invoke_rustc_compiler(candidate_rust_path);
        
        if comp_res.success {
            println!("   ✅ Compilation Succeeded at Iteration {}!", iteration);
            current_rust = std::fs::read_to_string(candidate_rust_path).unwrap_or(current_rust);
            return Ok(RepairResult {
                final_success: true,
                iterations_used: iteration,
                repair_logs: logs,
                final_rust_code: current_rust,
            });
        }

        println!("   ❌ rustc rejected candidate code with {} errors.", comp_res.error_count);

        // Package diagnostics
        let pkg = package_diagnostics(c_source, &current_rust, &comp_res.diagnostics, iteration);

        // Query LLM / Repair Agent
        let proposal = query_llm_repair_agent(&pkg)?;

        // Apply patch proposal
        let mut log_entry = apply_patch_to_file(candidate_rust_path, &proposal, iteration)?;
        log_entry.compilation_successful = false;
        logs.push(log_entry);

        current_rust = proposal.patched_rust_code;
    }

    // Final compilation check
    let final_comp = invoke_rustc_compiler(candidate_rust_path);
    current_rust = std::fs::read_to_string(candidate_rust_path).unwrap_or(current_rust);

    Ok(RepairResult {
        final_success: final_comp.success,
        iterations_used: max_iterations,
        repair_logs: logs,
        final_rust_code: current_rust,
    })
}
