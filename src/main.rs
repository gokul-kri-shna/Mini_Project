use std::env;
use std::path::Path;

pub mod data_stores;
pub mod module1_validation;
pub mod module2_ast;
pub mod module3_semantic;
pub mod module4_interprocedural;
pub mod module5_codegen;
pub mod module6_repair;
pub mod module7_gui;

fn main() {
    println!("=== C2SafeRust-LLM Transpilation Framework ===");

    let args: Vec<String> = env::args().collect();
    let gui_mode = args.iter().any(|arg| arg == "--gui");

    if gui_mode {
        println!("🚀 Launching egui Graphical Desktop User Interface...");
        if let Err(e) = module7_gui::launch_gui() {
            eprintln!("Failed to launch GUI: {}", e);
        }
        return;
    }

    // CLI Execution Mode
    let target_file = Path::new("benchmarks/sample_test.c");
    println!("\n--- Stage 1: Input Validation ---");
    let validated = match module1_validation::validate_input(target_file) {
        Ok(source) => {
            println!("✅ Input Validation Passed!");
            println!("   Path: {:?}", source.file_path);
            println!("   Size: {} bytes, {} lines", source.byte_size, source.line_count);
            source
        }
        Err(err) => {
            eprintln!("❌ Input Validation Failed: {}", err);
            return;
        }
    };

    println!("\n--- Stage 2: Preprocessing & AST Construction ---");
    let module2_output = match module2_ast::preprocess_and_parse(&validated) {
        Ok(output) => {
            println!("✅ Preprocessing & AST Construction Succeeded!");
            println!("   External GCC Used: {}", output.expanded_source.used_external_preprocessor);
            println!("   Functions Parsed: {}", output.ast.functions.len());
            output
        }
        Err(err) => {
            eprintln!("❌ Stage 2 Failed: {}", err);
            return;
        }
    };

    println!("\n--- Stage 3: Per-Function Semantic Analysis (Populating D1) ---");
    let d1_semantic_models = module3_semantic::analyze_translation_unit_semantics(&module2_output.ast);
    println!("✅ D1 Semantic Models Populated for {} functions!", d1_semantic_models.len());

    println!("\n--- Stage 4: Interprocedural Analysis (Populating D2) ---");
    let d2_summaries = module4_interprocedural::run_interprocedural_analysis(&module2_output.ast, &d1_semantic_models);
    println!("✅ D2 Interprocedural Summaries Populated!");

    println!("\n--- Stage 5: Ownership Classification & Rust Code Generation ---");
    let output_rs_path = Path::new("output.rs");
    let codegen_res = match module5_codegen::generate_rust_code(&module2_output.ast, &d1_semantic_models, &d2_summaries, output_rs_path) {
        Ok(res) => {
            println!("✅ Candidate Rust Code Generated Succeeded!");
            println!("   Output Written to: {}", res.output_file_path);
            res
        }
        Err(err) => {
            eprintln!("❌ Stage 5 Failed: {}", err);
            return;
        }
    };

    println!("\n--- Stage 6: Compiler Validation & LLM-Guided Repair ---");
    let repair_res = match module6_repair::run_repair_loop(&validated.raw_content, &codegen_res.output_file_path, 3) {
        Ok(res) => {
            println!("✅ Stage 6 Completed!");
            println!("   Final Compilation Success: {}", res.final_success);
            println!("   Iterations Used: {}", res.iterations_used);
            res
        }
        Err(err) => {
            eprintln!("❌ Stage 6 Failed: {}", err);
            return;
        }
    };

    println!("\n--- Stage 7: Report Assembly & Output Summary ---");
    let report = module7_gui::report_assembler::assemble_translation_report(
        &validated,
        &module2_output.ast,
        &d1_semantic_models,
        &d2_summaries,
        &repair_res,
    );

    println!("✅ Translation Report Assembled Successfully!");
    println!("\n{}", report.formatted_markdown_report);

    println!("\n💡 Tip: To launch the interactive egui Desktop GUI, run:");
    println!("   cargo run -- --gui");
}
