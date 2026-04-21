use biblia_egui::BibliaApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Minha Bíblia em Rust Desktop",
        options,
        Box::new(|_cc| Ok(Box::new(BibliaApp::default()))),
    )
}
