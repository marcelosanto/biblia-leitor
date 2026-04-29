use biblia_egui::BibliaApp;
use eframe::egui::{self};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 800.0])
            .with_min_inner_size([800.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Minha Bíblia em Rust Desktop",
        options,
        Box::new(|cc| Ok(Box::new(BibliaApp::new(cc)))),
    )
}
