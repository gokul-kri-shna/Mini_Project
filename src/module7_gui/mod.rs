pub mod report_assembler;
pub mod gui_app;

pub use report_assembler::{assemble_translation_report, TranslationReport, IdiomStatus};
pub use gui_app::C2SafeRustApp;

/// Primary entry point for Module 7: Launches the egui Graphical User Interface.
pub fn launch_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1100.0, 700.0])
            .with_title("C2SafeRust-LLM Transpilation Framework"),
        ..Default::default()
    };

    eframe::run_native(
        "C2SafeRust-LLM Transpilation Framework",
        options,
        Box::new(|cc| Ok(Box::new(C2SafeRustApp::new(cc)))),
    )
}
