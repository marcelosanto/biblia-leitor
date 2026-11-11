// ── Desktop (Linux / Windows / macOS) ──
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use biblia_egui::BibliaApp;
    use eframe::egui;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Bíblia Sagrada")
            .with_inner_size([900.0, 820.0])
            .with_min_inner_size([360.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Bíblia Sagrada",
        options,
        Box::new(|cc| Ok(Box::new(BibliaApp::new(cc)))),
    )
}

// ── Web (wasm32) – compilado pelo `trunk` ──
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    wasm_bindgen_futures::spawn_local(async {
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("the_canvas_id"))
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("canvas #the_canvas_id não encontrado no index.html");

        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(biblia_egui::BibliaApp::new(cc)))),
            )
            .await
            .expect("falha ao iniciar o app web");
    });
}
