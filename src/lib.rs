use eframe::egui;
use rusqlite::{Connection, Result};

mod db;

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

//ANDROID
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    use eframe::NativeOptions;

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let options = NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    eframe::run_native(
        "biblia_egui",
        options,
        Box::new(|_cc| Ok(Box::new(BibliaApp::default()))),
    )
    .unwrap();
}

#[derive(PartialEq)]
enum Tela {
    Leitura,
    Busca,
    Configuracoes,
}

pub struct BibliaApp {
    livro_selecionado: i32,
    nome_livro: String,
    capitulo: i32,
    versiculos: Vec<(i32, String)>,
}

impl Default for BibliaApp {
    fn default() -> Self {
        Self {
            livro_selecionado: 1,
            nome_livro: String::new(),
            capitulo: 1,
            versiculos: Vec::new(),
        }
    }
}

impl BibliaApp {
    fn carregar_capitulo(&mut self) {
        let path = crate::db::get_db_path();

        if let Ok(conn) = rusqlite::Connection::open(path) {
            let mut stmt = conn
                .prepare("SELECT verse, text FROM verses WHERE book = ?1 AND chapter = ?2")
                .unwrap();

            let iter = stmt
                .query_map([self.livro_selecionado, self.capitulo], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .unwrap();

            self.versiculos = iter.filter_map(|res| res.ok()).collect();
        }
    }
}

impl eframe::App for BibliaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {

        // 1. MENU LATERAL (DESKTOP)
        #[cfg(not(target_os = "android"))]
        egui::Panel::left("menu_livros").show_inside(ui, |ui| {
            ui.heading("Livros");
            egui::ScrollArea::vertical().show(ui, |ui| {
                //  fazer um SELECT na tabela 'books' para preencher isso
                if ui
                    .selectable_label(self.livro_selecionado == 1, "Gênesis")
                    .clicked()
                {
                    self.livro_selecionado = 1;
                    self.capitulo = 1;
                    self.carregar_capitulo();
                }
                .
            });
        });

        // 2. MENU INFERIOR (ANDROID)
        #[cfg(target_os = "android")]
        egui::Panel::bottom("navegacao_mobile").show_inside(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let largura_botao = ui.available_width() / 2.0;
                if ui
                    .add_sized([largura_botao, 45.0], egui::Button::new("⬅ Anterior"))
                    .clicked()
                {
                    if self.capitulo > 1 {
                        self.capitulo -= 1;
                        self.carregar_capitulo();
                    }
                }
                if ui
                    .add_sized([largura_botao, 45.0], egui::Button::new("Próximo ➡"))
                    .clicked()
                {
                    self.capitulo += 1;
                    self.carregar_capitulo();
                }
            });
        });

        // 3. ÁREA DE LEITURA
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading(format!("Capítulo {}", self.capitulo));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (num, texto) in &self.versiculos {
                    ui.horizontal_top(|ui| {
                        // Estiliza o número do versículo
                        ui.label(
                            egui::RichText::new(num.to_string())
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                        ui.label(texto);
                    });
                    ui.add_space(8.0); // Espaçamento entre versículos
                }
            });
        });
    }
}
