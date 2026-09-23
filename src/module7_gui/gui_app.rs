use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke, Vec2};
use rfd::FileDialog;

pub struct PipelineResult {
    pub file_path: String,
    pub original_c_code: String,
    pub generated_rust_code: String,
    pub markdown_report_text: String,
    pub c_lines: usize,
    pub c_bytes: usize,
    pub func_count: usize,
    pub call_nodes: usize,
    pub call_edges: usize,
    pub repair_iterations: usize,
    pub idioms_status_display: Vec<(usize, String, bool, String)>,
    pub repair_logs: Vec<(usize, String, String)>,
    pub is_success: bool,
    pub is_error: bool,
    pub status_message: String,
}

#[derive(Default)]
pub struct C2SafeRustApp {
    pub file_path_input: String,
    pub original_c_code: String,
    pub generated_rust_code: String,
    pub status_message: String,
    pub is_success: bool,
    pub is_error: bool,
    pub is_transpiling: bool,
    pub active_tab: usize, // 0 = 6 C Idioms, 1 = Executive Report, 2 = Repair History
    pub markdown_report_text: String,
    pub idioms_status_display: Vec<(usize, String, bool, String)>, // (id, name, detected, resolution)
    pub c_lines: usize,
    pub c_bytes: usize,
    pub func_count: usize,
    pub call_nodes: usize,
    pub call_edges: usize,
    pub repair_iterations: usize,
    pub repair_logs: Vec<(usize, String, String)>, // (iteration, source, note)
    pub transpilation_rx: Option<Receiver<PipelineResult>>,
}

impl C2SafeRustApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set custom dark theme visuals
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(18, 20, 29);
        visuals.window_fill = Color32::from_rgb(24, 27, 38);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(28, 32, 46);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(38, 44, 64);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(58, 66, 94);
        visuals.widgets.active.bg_fill = Color32::from_rgb(99, 102, 241); // Indigo
        cc.egui_ctx.set_visuals(visuals);

        let mut app = Self::default();
        app.file_path_input = "benchmarks/sample_test.c".to_string();
        app.load_file_from_path();
        app
    }

    /// Loads the C source file into the GUI preview without performing transpilation.
    pub fn load_file_from_path(&mut self) {
        let path = Path::new(&self.file_path_input);
        if path.exists() && path.is_file() {
            if let Ok(content) = std::fs::read_to_string(path) {
                self.original_c_code = content.clone();
                self.c_lines = content.lines().count();
                self.c_bytes = content.len();
                self.generated_rust_code = "// C file preview loaded successfully.\n// Click '▶ Run Transpilation' above to convert to Safe Rust.".to_string();
                self.markdown_report_text = "# Translation Report\n\n*Transpilation pending. Click '▶ Run Transpilation' to generate execution summary.*".to_string();
                self.idioms_status_display.clear();
                self.repair_logs.clear();
                self.func_count = 0;
                self.call_nodes = 0;
                self.call_edges = 0;
                self.repair_iterations = 0;
                self.is_success = false;
                self.is_error = false;
                self.status_message = format!("📂 Loaded C file '{}' ({} lines, {} bytes). Click '▶ Run Transpilation' to transpile.", self.file_path_input, self.c_lines, self.c_bytes);
                return;
            }
        }
        self.original_c_code = format!("// Error: File not found or unable to read at '{}'", self.file_path_input);
        self.c_lines = 0;
        self.c_bytes = 0;
        self.generated_rust_code = "".to_string();
        self.is_error = true;
        self.is_success = false;
        self.status_message = format!("⚠️ File not found or unreadable: '{}'", self.file_path_input);
    }

    /// Opens the native OS file selection dialog and displays the chosen file immediately in GUI preview.
    pub fn open_native_file_dialog(&mut self) {
        if let Some(file_path) = FileDialog::new()
            .set_title("Select C Source File for Transpilation")
            // .add_filter("C Source Files (*.c, *.h)", &["c", "h"])
            .pick_file()
        {
            self.file_path_input = file_path.to_string_lossy().to_string();
            self.load_file_from_path();
        }
    }

    /// Triggers transpilation asynchronously in a background worker thread.
    pub fn start_transpilation(&mut self) {
        if self.is_transpiling {
            return;
        }

        let path = Path::new(&self.file_path_input);
        if !path.exists() || !path.is_file() {
            self.load_file_from_path();
            return;
        }

        self.is_transpiling = true;
        self.is_success = false;
        self.is_error = false;
        self.status_message = format!("⚡ Transpiling '{}' in background (AST, Semantic Analysis, Codegen, Repair)...", self.file_path_input);

        let (tx, rx) = channel();
        self.transpilation_rx = Some(rx);
        let path_str = self.file_path_input.clone();

        std::thread::spawn(move || {
            let result = execute_transpilation_pipeline(&path_str);
            let _ = tx.send(result);
        });
    }
}

/// Standalone worker function that executes Stages 1-7 of the transpilation pipeline.
fn execute_transpilation_pipeline(path_str: &str) -> PipelineResult {
    let path = PathBuf::from(path_str);

    // Stage 1: Input Validation
    let validated = match crate::module1_validation::validate_input(&path) {
        Ok(src) => src,
        Err(err) => {
            return PipelineResult {
                file_path: path_str.to_string(),
                original_c_code: String::new(),
                generated_rust_code: String::new(),
                markdown_report_text: format!("# Validation Error\n\n{}", err),
                c_lines: 0,
                c_bytes: 0,
                func_count: 0,
                call_nodes: 0,
                call_edges: 0,
                repair_iterations: 0,
                idioms_status_display: vec![],
                repair_logs: vec![],
                is_success: false,
                is_error: true,
                status_message: format!("❌ Validation Error: {}", err),
            };
        }
    };

    let original_c_code = validated.raw_content.clone();
    let c_lines = validated.line_count;
    let c_bytes = validated.byte_size;

    // Stage 2: Preprocessing & AST Construction
    let module2_out = match crate::module2_ast::preprocess_and_parse(&validated) {
        Ok(out) => out,
        Err(err) => {
            return PipelineResult {
                file_path: path_str.to_string(),
                original_c_code,
                generated_rust_code: String::new(),
                markdown_report_text: format!("# AST Error\n\n{}", err),
                c_lines,
                c_bytes,
                func_count: 0,
                call_nodes: 0,
                call_edges: 0,
                repair_iterations: 0,
                idioms_status_display: vec![],
                repair_logs: vec![],
                is_success: false,
                is_error: true,
                status_message: format!("❌ AST Error: {}", err),
            };
        }
    };

    // Stage 3: Per-Function Semantic Analysis
    let d1 = crate::module3_semantic::analyze_translation_unit_semantics(&module2_out.ast);

    // Stage 4: Interprocedural Analysis
    let d2 = crate::module4_interprocedural::run_interprocedural_analysis(&module2_out.ast, &d1);

    // Stage 5: Ownership Classification & Rust Codegen
    let out_rs = Path::new("output.rs");
    let codegen_res = match crate::module5_codegen::generate_rust_code(&module2_out.ast, &d1, &d2, out_rs) {
        Ok(res) => res,
        Err(err) => {
            return PipelineResult {
                file_path: path_str.to_string(),
                original_c_code,
                generated_rust_code: String::new(),
                markdown_report_text: format!("# Codegen Error\n\n{}", err),
                c_lines,
                c_bytes,
                func_count: module2_out.ast.functions.len(),
                call_nodes: 0,
                call_edges: 0,
                repair_iterations: 0,
                idioms_status_display: vec![],
                repair_logs: vec![],
                is_success: false,
                is_error: true,
                status_message: format!("❌ Codegen Error: {}", err),
            };
        }
    };

    // Stage 6: Compiler Validation & LLM Repair Loop
    let repair_res = match crate::module6_repair::run_repair_loop(&validated.raw_content, &codegen_res.output_file_path, 3) {
        Ok(res) => res,
        Err(err) => {
            return PipelineResult {
                file_path: path_str.to_string(),
                original_c_code,
                generated_rust_code: String::new(),
                markdown_report_text: format!("# Repair Loop Error\n\n{}", err),
                c_lines,
                c_bytes,
                func_count: module2_out.ast.functions.len(),
                call_nodes: 0,
                call_edges: 0,
                repair_iterations: 0,
                idioms_status_display: vec![],
                repair_logs: vec![],
                is_success: false,
                is_error: true,
                status_message: format!("❌ Repair Loop Error: {}", err),
            };
        }
    };

    // Stage 7: Report Assembly
    let report = crate::module7_gui::report_assembler::assemble_translation_report(
        &validated,
        &module2_out.ast,
        &d1,
        &d2,
        &repair_res,
    );

    let idioms_status_display = report
        .idioms_summary
        .iter()
        .map(|i| (i.idiom_number, i.name.to_string(), i.detected, i.resolution.clone()))
        .collect();

    let repair_logs = repair_res
        .repair_logs
        .iter()
        .map(|l| (l.iteration, l.patch_source.clone(), l.explanation.clone()))
        .collect();

    let status_message = if report.compilation_successful {
        "✅ Transpilation & Compiler Verification Succeeded!".to_string()
    } else {
        format!("⚠️ Transpilation Completed (Repair Iterations: {}).", report.repair_iterations)
    };

    PipelineResult {
        file_path: path_str.to_string(),
        original_c_code: validated.raw_content,
        generated_rust_code: report.final_rust_code,
        markdown_report_text: report.formatted_markdown_report,
        c_lines: report.c_lines,
        c_bytes: report.c_bytes,
        func_count: report.functions_count,
        call_nodes: report.call_graph_nodes,
        call_edges: report.call_graph_edges,
        repair_iterations: report.repair_iterations,
        idioms_status_display,
        repair_logs,
        is_success: report.compilation_successful,
        is_error: false,
        status_message,
    }
}

impl eframe::App for C2SafeRustApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll background transpilation thread result
        if let Some(ref rx) = self.transpilation_rx {
            if let Ok(res) = rx.try_recv() {
                self.original_c_code = res.original_c_code;
                self.generated_rust_code = res.generated_rust_code;
                self.markdown_report_text = res.markdown_report_text;
                self.c_lines = res.c_lines;
                self.c_bytes = res.c_bytes;
                self.func_count = res.func_count;
                self.call_nodes = res.call_nodes;
                self.call_edges = res.call_edges;
                self.repair_iterations = res.repair_iterations;
                self.idioms_status_display = res.idioms_status_display;
                self.repair_logs = res.repair_logs;
                self.is_success = res.is_success;
                self.is_error = res.is_error;
                self.status_message = res.status_message;
                self.is_transpiling = false;
                self.transpilation_rx = None;
            }
        }

        if self.is_transpiling {
            ctx.request_repaint();
        }

        // --- Top Header Panel ---
        egui::TopBottomPanel::top("top_panel")
            .frame(
                Frame::none()
                    .fill(Color32::from_rgb(24, 27, 38))
                    .inner_margin(Margin::symmetric(16.0, 12.0))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(42, 47, 66))),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // App Logo / Title
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚡").size(22.0));
                        ui.label(
                            RichText::new("C2SafeRust-LLM")
                                .strong()
                                .size(20.0)
                                .color(Color32::from_rgb(129, 140, 248)),
                        );
                        ui.label(RichText::new("v1.0").size(11.0).color(Color32::from_rgb(156, 163, 175)));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Transpile Action Button
                        if self.is_transpiling {
                            let disabled_btn = egui::Button::new(
                                RichText::new("⏳ Transpiling...")
                                    .strong()
                                    .size(14.0)
                                    .color(Color32::from_rgb(209, 213, 219)),
                            )
                            .fill(Color32::from_rgb(79, 70, 229))
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(160.0, 36.0));
                            ui.add_enabled(false, disabled_btn);
                        } else {
                            let btn = egui::Button::new(
                                RichText::new("▶ Run Transpilation")
                                    .strong()
                                    .size(14.0)
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(99, 102, 241))
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(160.0, 36.0));

                            if ui.add(btn).clicked() {
                                self.start_transpilation();
                            }
                        }

                        ui.add_space(8.0);

                        // Native File Dialog Button ("📂 Browse File...")
                        let browse_btn = egui::Button::new(
                            RichText::new("📂 Browse File...")
                                .strong()
                                .size(13.0)
                                .color(Color32::from_rgb(229, 231, 235)),
                        )
                        .fill(Color32::from_rgb(47, 53, 76))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(130.0, 36.0));

                        if ui.add(browse_btn).clicked() {
                            self.open_native_file_dialog();
                        }

                        ui.add_space(8.0);

                        // Load File Button
                        let load_btn = egui::Button::new(
                            RichText::new("📄 Load")
                                .strong()
                                .size(13.0)
                                .color(Color32::from_rgb(229, 231, 235)),
                        )
                        .fill(Color32::from_rgb(38, 44, 64))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(70.0, 36.0));

                        if ui.add(load_btn).clicked() {
                            self.load_file_from_path();
                        }

                        ui.add_space(8.0);

                        // File Path Text Input Box
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("C Source:").size(13.0).color(Color32::from_rgb(209, 213, 219)));
                            let text_response = ui.add(
                                egui::TextEdit::singleline(&mut self.file_path_input)
                                    .desired_width(240.0)
                                    .font(egui::TextStyle::Monospace),
                            );
                            if text_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                self.load_file_from_path();
                            }
                        });
                    });
                });

                ui.add_space(6.0);

                // Status Banner Card
                let status_bg = if self.is_transpiling {
                    Color32::from_rgb(99, 102, 241) // Indigo
                } else if self.is_success {
                    Color32::from_rgb(16, 185, 129) // Emerald Green
                } else if self.is_error {
                    Color32::from_rgb(239, 68, 68) // Red
                } else {
                    Color32::from_rgb(38, 44, 64) // Dark Neutral
                };

                Frame::none()
                    .fill(status_bg)
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::symmetric(12.0, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if self.is_transpiling {
                                ui.spinner();
                            }
                            ui.label(RichText::new(&self.status_message).strong().size(13.0).color(Color32::WHITE));
                        });
                    });
            });

        // --- KPI Metric Summary Bar ---
        egui::TopBottomPanel::top("kpi_panel")
            .frame(
                Frame::none()
                    .fill(Color32::from_rgb(18, 20, 29))
                    .inner_margin(Margin::symmetric(16.0, 8.0))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 44, 64))),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    render_kpi_card(ui, "📄 C File", &format!("{} lines", self.c_lines), &format!("{} bytes", self.c_bytes));
                    ui.add_space(12.0);
                    render_kpi_card(ui, "⚙️ AST Functions", &format!("{} parsed", self.func_count), "Translation Unit");
                    ui.add_space(12.0);
                    render_kpi_card(ui, "🕸️ Call Graph", &format!("{} nodes", self.call_nodes), &format!("{} edges", self.call_edges));
                    ui.add_space(12.0);
                    render_kpi_card(ui, "🤖 LLM Repair", &format!("{} iterations", self.repair_iterations), if self.is_success { "Verified" } else if self.is_transpiling { "Processing..." } else { "Idle" });
                });
            });

        // --- Bottom Drawer Panel (Resizing Drawer with Report & Tabs) ---
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .default_height(280.0)
            .height_range(120.0..=650.0)
            .frame(
                Frame::none()
                    .fill(Color32::from_rgb(24, 27, 38))
                    .inner_margin(Margin::symmetric(16.0, 10.0))
                    .stroke(Stroke::new(1.5_f32, Color32::from_rgb(99, 102, 241))),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.active_tab, 0, RichText::new("📊 6 C Idioms Taxonomy").strong());
                    ui.selectable_value(&mut self.active_tab, 1, RichText::new("📜 Executive Verification Report").strong());
                    ui.selectable_value(&mut self.active_tab, 2, RichText::new("🛠️ Repair History Trace").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("↕ Drag top border to resize bottom drawer").size(11.0).color(Color32::from_rgb(156, 163, 175)));
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.active_tab {
                        0 => render_idioms_tab(ui, &self.idioms_status_display),
                        1 => render_executive_report_tab(ui, self),
                        _ => render_repair_logs_tab(ui, &self.repair_logs),
                    }
                });
            });

        // --- Resizable Resizing Side-by-Side Code Panels ---
        // Left Side Panel: C Source Code (Resizable horizontally, like VS Code split editor)
        egui::SidePanel::left("left_c_code_panel")
            .resizable(true)
            .default_width(ctx.screen_rect().width() * 0.48)
            .min_width(240.0)
            .frame(
                Frame::none()
                    .fill(Color32::from_rgb(15, 17, 26))
                    .inner_margin(Margin::same(12.0))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(42, 47, 66))),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Original C Source Code (.c)").strong().size(14.0).color(Color32::from_rgb(147, 197, 253)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("↔ Drag divider to resize split view").size(11.0).color(Color32::from_rgb(156, 163, 175)));
                    });
                });
                ui.separator();
                egui::ScrollArea::both().id_source("c_code_scroll").show(ui, |ui| {
                    render_syntax_highlighted_code(ui, &self.original_c_code, false);
                });
            });

        // Right Central Panel: Generated Safe Rust Code (Takes remaining right space)
        egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(Color32::from_rgb(15, 17, 26))
                    .inner_margin(Margin::same(12.0))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(42, 47, 66))),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Generated Safe Rust Code (.rs)").strong().size(14.0).color(Color32::from_rgb(167, 243, 208)));
                });
                ui.separator();
                egui::ScrollArea::both().id_source("rust_code_scroll").show(ui, |ui| {
                    render_syntax_highlighted_code(ui, &self.generated_rust_code, true);
                });
            });
    }
}

fn render_kpi_card(ui: &mut egui::Ui, title: &str, main_val: &str, sub_val: &str) {
    Frame::none()
        .fill(Color32::from_rgb(28, 32, 46))
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(42, 47, 66)))
        .inner_margin(Margin::symmetric(14.0, 8.0))
        .show(ui, |ui| {
            ui.label(RichText::new(title).size(11.0).color(Color32::from_rgb(156, 163, 175)));
            ui.label(RichText::new(main_val).strong().size(14.0).color(Color32::WHITE));
            ui.label(RichText::new(sub_val).size(10.0).color(Color32::from_rgb(129, 140, 248)));
        });
}

fn render_idioms_tab(ui: &mut egui::Ui, items: &[(usize, String, bool, String)]) {
    ui.add_space(6.0);
    egui::Grid::new("idioms_grid_custom")
        .striped(true)
        .min_col_width(120.0)
        .spacing(Vec2::new(16.0, 8.0))
        .show(ui, |ui| {
            ui.label(RichText::new("ID").strong().color(Color32::from_rgb(156, 163, 175)));
            ui.label(RichText::new("C Ownership Idiom").strong().color(Color32::from_rgb(156, 163, 175)));
            ui.label(RichText::new("Status").strong().color(Color32::from_rgb(156, 163, 175)));
            ui.label(RichText::new("Resolution Mechanism").strong().color(Color32::from_rgb(156, 163, 175)));
            ui.end_row();

            let default_idioms = [
                (1, "Aliased &mut parameters", "Interprocedural points-to analysis"),
                (2, "T** consumes ownership", "Callee write-through def-use link"),
                (3, "T** creates ownership", "Callee heap allocation summary"),
                (4, "Overlapping pointer-arithmetic slices", "Call-site range overlap check"),
                (5, "Union active variant across boundary", "Interprocedural reaching-definitions"),
                (6, "Data-dependent ownership (runtime)", "Option<Box<T>> / LLM Repair Agent"),
            ];

            if items.is_empty() {
                for (id, name, res) in &default_idioms {
                    ui.label(id.to_string());
                    ui.label(*name);
                    ui.label(RichText::new("PENDING").color(Color32::from_rgb(156, 163, 175)));
                    ui.label(*res);
                    ui.end_row();
                }
            } else {
                for (id, name, detected, res) in items {
                    ui.label(id.to_string());
                    ui.label(name);
                    if *detected {
                        ui.label(RichText::new("DETECTED").strong().color(Color32::from_rgb(245, 158, 11))); // Amber
                    } else {
                        ui.label(RichText::new("SAFE").color(Color32::from_rgb(16, 185, 129))); // Green
                    }
                    ui.label(res);
                    ui.end_row();
                }
            }
        });
}

/// Renders a formal executive verification report document with styled sections, cards, and taxonomy breakdown.
fn render_executive_report_tab(ui: &mut egui::Ui, app: &C2SafeRustApp) {
    ui.add_space(8.0);

    // --- Formal Document Header ---
    Frame::none()
        .fill(Color32::from_rgb(20, 24, 38))
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(99, 102, 241)))
        .inner_margin(Margin::same(14.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📜 C2SafeRust-LLM Verification & Analysis Report").strong().size(16.0).color(Color32::from_rgb(129, 140, 248)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.is_success {
                        ui.label(RichText::new(" VERIFIED SAFE RUST ").strong().size(12.0).background_color(Color32::from_rgb(16, 185, 129)).color(Color32::WHITE));
                    } else if app.is_transpiling {
                        ui.label(RichText::new(" TRANSPILING IN PROGRESS ").strong().size(12.0).background_color(Color32::from_rgb(99, 102, 241)).color(Color32::WHITE));
                    } else {
                        ui.label(RichText::new(" TRANSPILATION PENDING ").strong().size(12.0).background_color(Color32::from_rgb(245, 158, 11)).color(Color32::WHITE));
                    }
                });
            });

            ui.add_space(4.0);
            ui.label(RichText::new(format!("Target File: {}  |  Generated on: Local Session", app.file_path_input)).size(12.0).color(Color32::from_rgb(156, 163, 175)));
        });

    ui.add_space(10.0);

    // --- Executive Metrics Summary Cards ---
    ui.columns(4, |cols| {
        Frame::none().fill(Color32::from_rgb(28, 32, 46)).rounding(Rounding::same(6.0)).inner_margin(Margin::same(10.0)).show(&mut cols[0], |ui| {
            ui.label(RichText::new("📄 Source File").size(11.0).color(Color32::from_rgb(156, 163, 175)));
            ui.label(RichText::new(format!("{} lines", app.c_lines)).strong().size(14.0).color(Color32::WHITE));
            ui.label(RichText::new(format!("{} bytes", app.c_bytes)).size(10.0).color(Color32::from_rgb(147, 197, 253)));
        });

        Frame::none().fill(Color32::from_rgb(28, 32, 46)).rounding(Rounding::same(6.0)).inner_margin(Margin::same(10.0)).show(&mut cols[1], |ui| {
            ui.label(RichText::new("⚙️ AST Functions").size(11.0).color(Color32::from_rgb(156, 163, 175)));
            ui.label(RichText::new(format!("{} functions", app.func_count)).strong().size(14.0).color(Color32::WHITE));
            ui.label(RichText::new("Parsed Translation Unit").size(10.0).color(Color32::from_rgb(147, 197, 253)));
        });

        Frame::none().fill(Color32::from_rgb(28, 32, 46)).rounding(Rounding::same(6.0)).inner_margin(Margin::same(10.0)).show(&mut cols[2], |ui| {
            ui.label(RichText::new("🕸️ Call Graph").size(11.0).color(Color32::from_rgb(156, 163, 175)));
            ui.label(RichText::new(format!("{} nodes", app.call_nodes)).strong().size(14.0).color(Color32::WHITE));
            ui.label(RichText::new(format!("{} edges", app.call_edges)).size(10.0).color(Color32::from_rgb(147, 197, 253)));
        });

        Frame::none().fill(Color32::from_rgb(28, 32, 46)).rounding(Rounding::same(6.0)).inner_margin(Margin::same(10.0)).show(&mut cols[3], |ui| {
            ui.label(RichText::new("🤖 Repair Loop").size(11.0).color(Color32::from_rgb(156, 163, 175)));
            ui.label(RichText::new(format!("{} iterations", app.repair_iterations)).strong().size(14.0).color(Color32::WHITE));
            ui.label(RichText::new(if app.is_success { "Verified Compiler Pass" } else { "Pending Verification" }).size(10.0).color(Color32::from_rgb(16, 185, 129)));
        });
    });

    ui.add_space(12.0);

    // --- Section 1: Formal Idiom Safety Audit ---
    ui.label(RichText::new("1. Ownership & Safety Idioms Audit").strong().size(14.0).color(Color32::from_rgb(229, 231, 235)));
    ui.add_space(4.0);
    render_idioms_tab(ui, &app.idioms_status_display);

    ui.add_space(14.0);

    // --- Section 2: Technical Summary Report Output ---
    ui.label(RichText::new("2. Raw Translation Report Document").strong().size(14.0).color(Color32::from_rgb(229, 231, 235)));
    ui.add_space(4.0);

    Frame::none()
        .fill(Color32::from_rgb(18, 20, 29))
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(42, 47, 66)))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(&app.markdown_report_text)
                    .font(egui::FontId::monospace(12.0))
                    .color(Color32::from_rgb(209, 213, 219)),
            );
        });
}

fn render_repair_logs_tab(ui: &mut egui::Ui, logs: &[(usize, String, String)]) {
    ui.add_space(6.0);
    if logs.is_empty() {
        ui.label(RichText::new("No repair iterations logged yet.").italics());
    } else {
        for (iter, source, note) in logs {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Iteration {}:", iter)).strong().color(Color32::from_rgb(129, 140, 248)));
                ui.label(RichText::new(format!("[Source: {}]", source)).strong().color(Color32::from_rgb(16, 185, 129)));
                ui.label(note);
            });
            ui.separator();
        }
    }
}

/// Renders compiler-style syntax highlighted code line-by-line with horizontal and vertical scroll support
fn render_syntax_highlighted_code(ui: &mut egui::Ui, code: &str, is_rust: bool) {
    if code.trim().is_empty() {
        ui.label(RichText::new("// No code to display").color(Color32::from_rgb(107, 114, 128)).font(egui::FontId::monospace(13.0)));
        return;
    }

    for (line_idx, line) in code.lines().enumerate() {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{:3} │ ", line_idx + 1))
                    .color(Color32::from_rgb(107, 114, 128))
                    .font(egui::FontId::monospace(13.0)),
            );
            render_tokens_for_line(ui, line, is_rust);
        });
    }
}

fn render_tokens_for_line(ui: &mut egui::Ui, line: &str, is_rust: bool) {
    let trimmed = line.trim();

    if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
        ui.label(RichText::new(line).color(Color32::from_rgb(106, 153, 85)).font(egui::FontId::monospace(13.0)));
        return;
    }
    if trimmed.starts_with('#') {
        ui.label(RichText::new(line).color(Color32::from_rgb(156, 163, 175)).font(egui::FontId::monospace(13.0)));
        return;
    }

    let tokens = tokenize_code_line(line);
    for token in tokens {
        let color = get_token_color(&token, is_rust);
        ui.label(RichText::new(token).color(color).font(egui::FontId::monospace(13.0)));
    }
}

fn tokenize_code_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for c in line.chars() {
        if c.is_alphanumeric() || c == '_' {
            current.push(c);
        } else {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            tokens.push(c.to_string());
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn get_token_color(token: &str, _is_rust: bool) -> Color32 {
    let tok = token.trim();

    let c_keywords = ["int", "char", "float", "double", "void", "struct", "union", "return", "if", "else", "for", "while", "typedef", "sizeof", "NULL"];
    let rust_keywords = ["pub", "fn", "let", "mut", "struct", "enum", "return", "if", "else", "for", "while", "match", "use", "mod", "impl", "Box", "Vec", "Option"];

    if c_keywords.contains(&tok) || rust_keywords.contains(&tok) {
        return Color32::from_rgb(197, 134, 192);
    }

    let types = ["i32", "u8", "f32", "f64", "i64", "i16", "usize", "isize", "bool", "String", "str"];
    if types.contains(&tok) {
        return Color32::from_rgb(78, 201, 176);
    }

    let functions = ["main", "swap", "printf", "scanf", "malloc", "free", "println", "format", "create_buffer", "release_buffer", "combine_slices", "update_twin_counters"];
    if functions.contains(&tok) {
        return Color32::from_rgb(220, 220, 170);
    }

    if tok.chars().all(|c| c.is_numeric()) {
        return Color32::from_rgb(206, 145, 120);
    }

    if tok == "*" || tok == "&" || tok == "->" || tok == "**" {
        return Color32::from_rgb(244, 63, 94);
    }

    Color32::from_rgb(212, 212, 212)
}
