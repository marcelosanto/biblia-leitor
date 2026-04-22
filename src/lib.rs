use eframe::egui::{self, RichText, Visuals};
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
        Box::new(|cc| Ok(Box::new(BibliaApp::new(cc)))),
    )
    .unwrap();
}

#[derive(PartialEq)]
enum Tela {
    Leitura,
    Busca,
    Configuracoes,
}

#[derive(Clone)]
pub struct Livro {
    pub id: i32,
    pub name: String,
    pub abbrev: String,
}

pub struct BibliaApp {
    livro_selecionado: i32,
    nome_livro: String,
    lista_livros: Vec<Livro>,
    capitulo: i32,
    versiculos: Vec<(i32, String)>,
}

impl BibliaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configura_context(&cc.egui_ctx);

        let mut app = Self {
            livro_selecionado: 1,
            nome_livro: "Gênesis".to_string(),
            lista_livros: Vec::new(),
            capitulo: 1,
            versiculos: Vec::new(),
        };

        app.carregar_lista_livros();
        app.carregar_capitulo();

        app
    }

    fn configura_context(ctx: &egui::Context) {
        ctx.set_visuals(Visuals::light());
        //  ctx.set_pixels_per_point(2.0);
        let mut style = (*ctx.global_style()).clone();
        // Aumenta o texto para facilitar a leitura no celular
        style
            .text_styles
            .get_mut(&egui::TextStyle::Body)
            .unwrap()
            .size = 18.0;
        style
            .text_styles
            .get_mut(&egui::TextStyle::Button)
            .unwrap()
            .size = 16.0;

        ctx.set_global_style(style);
    }

    fn carregar_lista_livros(&mut self) {
        let path = crate::db::get_db_path();
        if let Ok(conn) = rusqlite::Connection::open(path) {
            let mut stmt = conn
                .prepare("SELECT id, name, abbrev FROM books ORDER BY id")
                .unwrap();

            let livros_iter = stmt
                .query_map([], |row| {
                    Ok(Livro {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        abbrev: row.get(2)?,
                    })
                })
                .unwrap();

            self.lista_livros = livros_iter.filter_map(|res| res.ok()).collect();
        }
    }

    fn total_capitulos_do_livro(&self, livro_id: i32) -> i32 {
        let path = crate::db::get_db_path();
        if let Ok(conn) = rusqlite::Connection::open(path) {
            // Procuramos o maior número de capítulo para o livro atual
            let mut stmt = conn
                .prepare("SELECT MAX(chapter) FROM verses WHERE book = ?1")
                .unwrap();

            // O query_row retorna apenas um resultado
            if let Ok(total) = stmt.query_row([livro_id], |row| row.get::<_, i32>(0)) {
                return total;
            }
        }
        1 // Valor padrão caso algo falhe
    }

    fn carregar_capitulo(&mut self) {
        let path = crate::db::get_db_path();

        match Connection::open(&path) {
            Ok(conn) => {
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
            Err(e) => {
                eprintln!("Erro ao abrir banco em {:?}: {}", path, e);
            }
        }
    }

    // fn carregar_capitulo(&mut self) {
    //     let path = crate::db::get_db_path();

    //     if let Ok(conn) = rusqlite::Connection::open(path) {
    //         let mut stmt = conn
    //             .prepare("SELECT verse, text FROM verses WHERE book = ?1 AND chapter = ?2")
    //             .unwrap();

    //         let iter = stmt
    //             .query_map([self.livro_selecionado, self.capitulo], |row| {
    //                 Ok((row.get(0)?, row.get(1)?))
    //             })
    //             .unwrap();

    //         self.versiculos = iter.filter_map(|res| res.ok()).collect();
    //     }
    // }
    //
    fn livro_anterior(&mut self) {
        if self.capitulo > 1 {
            self.capitulo -= 1;
            self.carregar_capitulo();
        }
    }

    fn proximo_livro(&mut self) {
        self.capitulo += 1;
        self.carregar_capitulo();
    }
}

impl eframe::App for BibliaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 1. MENU LATERAL (DESKTOP)
        #[cfg(not(target_os = "android"))]
        egui::Panel::left("menu_livros").show_inside(ui, |ui| {
            ui.heading("Livros");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                //  fazer um SELECT na tabela 'books' para preencher isso
                for livro in &self.lista_livros.clone() {
                    let is_selected = self.livro_selecionado == livro.id;
                    if ui.selectable_label(is_selected, &livro.name).clicked() {
                        self.livro_selecionado = livro.id;
                        self.nome_livro = livro.name.clone();
                        self.capitulo = 1;
                        self.carregar_capitulo();
                    }
                }
            });
        });

        // 2. MENU INFERIOR (ANDROID)
        #[cfg(target_os = "android")]
        egui::Panel::bottom("navegacao_mobile").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("")
                    .selected_text(&self.nome_livro)
                    .show_ui(ui, |ui| {
                        for livro in &self.lista_livros.clone() {
                            if ui
                                .selectable_value(
                                    &mut self.livro_selecionado,
                                    livro.id,
                                    &livro.name,
                                )
                                .clicked()
                            {
                                self.nome_livro = livro.name.clone();
                                self.capitulo = 1;
                                self.carregar_capitulo();
                            }
                        }
                    });
                if ui
                    .add_enabled(self.capitulo > 1, egui::Button::new("⬅ Anterior"))
                    .clicked()
                {
                    // if self.capitulo > 1 {
                    //     self.capitulo -= 1;
                    //     self.carregar_capitulo();
                    // }
                    self.livro_anterior();
                }
                let n_capitulos = self.total_capitulos_do_livro(self.livro_selecionado);

                if ui
                    .add_enabled(self.capitulo < n_capitulos, egui::Button::new("Próximo ➡"))
                    .clicked()
                {
                    // self.capitulo += 1;
                    // self.carregar_capitulo();
                    self.proximo_livro();
                }
            });
        });

        // 3. ÁREA DE LEITURA
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading(format!("Capítulo {}", self.capitulo));
            ui.separator();

            let altura_do_texto = ui.text_style_height(&egui::TextStyle::Body);
            let total_versiculos = self.versiculos.len();

            egui::ScrollArea::vertical().show_rows(
                ui,
                altura_do_texto,
                total_versiculos,
                |ui, _range| {
                    // 'range' contém apenas os índices visíveis, ex: 100..115
                    for (num, texto) in &self.versiculos {
                        ui.label(format!("{num}: {texto}"));
                    }
                },
            );

            //     egui::ScrollArea::vertical().show(ui, |ui| {
            //         for (num, texto) in &self.versiculos {
            //             ui.horizontal_top(|ui| {
            //                 // Estiliza o número do versículo
            //                 ui.label(
            //                     egui::RichText::new(num.to_string())
            //                         .small()
            //                         .color(egui::Color32::GRAY),
            //                 );
            //                 ui.label(texto);
            //             });
            //             ui.add_space(8.0); // Espaçamento entre versículos
            //         }
            //     });
        });
    }
}
