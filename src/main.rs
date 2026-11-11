use biblia_egui::BibliaApp;
use eframe::egui::{self};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Bíblia Sagrada")
            .with_inner_size([900.0, 820.0])
            // Mínimo de "celular": a UI é responsiva, não precisa travar em 800x800
            .with_min_inner_size([360.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Bíblia Sagrada",
        options,
        Box::new(|cc| Ok(Box::new(BibliaApp::new(cc)))),
    )
}
