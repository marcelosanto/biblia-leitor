use eframe::{
    App,
    egui::{self, RichText, Visuals},
};
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

pub struct Versiculo {
    pub numero: i32,
    pub numero_formatado: String,
    pub texto: String,
}

pub struct BibliaApp {
    tela_atual: Tela,
    livro_selecionado: i32,
    nome_livro: String,
    lista_livros: Vec<Livro>,
    capitulo: i32,
    versiculos: Vec<Versiculo>,
    menu_aberto: bool,
    capitulo_mudou: bool,
    aguardando_saida: bool,
    historico: Vec<Tela>,
}

impl BibliaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configura_context(&cc.egui_ctx);

        let mut app = Self {
            tela_atual: Tela::Leitura,
            livro_selecionado: 1,
            nome_livro: "Gênesis".to_string(),
            lista_livros: Vec::new(),
            capitulo: 1,
            versiculos: Vec::new(),
            menu_aberto: false,
            capitulo_mudou: false,
            aguardando_saida: false,
            historico: Vec::new(),
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
                        let num: i32 = row.get(0)?;
                        let texto: String = row.get(1)?;

                        // Converte aqui, apenas uma vez por carregamento
                        let num_f = self.formatar_elevado(&num);

                        Ok(Versiculo {
                            numero: num,
                            numero_formatado: num_f,
                            texto: texto,
                        })
                    })
                    .unwrap();

                self.versiculos = iter.filter_map(|res| res.ok()).collect();
            }
            Err(e) => {
                eprintln!("Erro ao abrir banco em {:?}: {}", path, e);
            }
        }
    }

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

    fn formatar_elevado(&self, valor: &i32) -> String {
        valor
            .to_string()
            .chars()
            .map(|c| match c {
                '0' => '⁰',
                '1' => '¹',
                '2' => '²',
                '3' => '³',
                '4' => '⁴',
                '5' => '⁵',
                '6' => '⁶',
                '7' => '⁷',
                '8' => '⁸',
                '9' => '⁹',
                _ => c,
            })
            .collect()
    }

    fn navegar_para(&mut self, nova_tela: Tela) {
        if self.tela_atual != nova_tela {
            let antiga = std::mem::replace(&mut self.tela_atual, nova_tela);
            self.historico.push(antiga);
        }
    }

    fn voltar(&mut self) {
        if let Some(tela_anterior) = self.historico.pop() {
            self.tela_atual = tela_anterior;
        }
    }

    fn renderizar_header(&mut self, ui: &mut egui::Ui) {
        let mut top_frame = egui::Frame::NONE
            .fill(ui.visuals().window_fill())
            .inner_margin(egui::Margin::same(16));

        let mut margin = top_frame.inner_margin;

        #[cfg(target_os = "android")]
        {
            margin.top = 30;
            margin.bottom = 10;
        }

        top_frame.inner_margin = margin;

        egui::Panel::top("header")
            .frame(top_frame)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("☰").size(20.0)).clicked() {
                        self.menu_aberto = !self.menu_aberto;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(egui::RichText::new("🔍").size(20.0)).clicked() {
                            self.navegar_para(Tela::Busca);
                        }
                        if ui.button(egui::RichText::new("🔍").size(20.0)).clicked() {
                            self.navegar_para(Tela::Configuracoes);
                        }

                        ui.centered_and_justified(|ui| {
                            ui.heading("Bíblia Sagrada");
                        });
                    });
                });
            });
    }

    fn renderizar_menu(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("menu_livros").show_inside(ui, |ui| {
            ui.heading("Livros");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for livro in &self.lista_livros.clone() {
                    let is_selected = self.livro_selecionado == livro.id;
                    if ui.selectable_label(is_selected, &livro.name).clicked() {
                        self.livro_selecionado = livro.id;
                        self.nome_livro = livro.name.clone();
                        self.capitulo = 1;
                        self.carregar_capitulo();
                        self.capitulo_mudou = true;
                        self.menu_aberto = false;
                        self.navegar_para(Tela::Leitura);
                    }
                }
            });
        });
    }

    fn ui_leitura(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                // Isso centraliza o grupo horizontal dentro da largura disponível
                let total_width = ui.available_width();

                // Estimativa de largura do conjunto
                let group_width = 220.0;
                ui.add_space((total_width - group_width) / 2.0);

                // --- Botão Esquerdo (Seta minimalista) ---
                let btn_prev = egui::Button::new(
                    egui::RichText::new("<")
                        .size(24.0)
                        .color(egui::Color32::from_rgb(138, 154, 91)),
                )
                .frame(false);
                if ui.add_enabled(self.capitulo > 1, btn_prev).clicked() {
                    self.livro_anterior();
                    self.capitulo_mudou = true;
                }

                ui.add_space(20.0); // Espaço entre seta e texto

                // --- Título: Nome (Preto) e Capítulo (Cinza) ---
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&self.nome_livro)
                            .heading()
                            .strong()
                            .color(ui.visuals().widgets.active.fg_stroke.color),
                    ); // Cor principal

                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new(self.capitulo.to_string())
                            .heading()
                            .color(egui::Color32::from_gray(140)),
                    ); // Cor cinza da imagem
                });

                ui.add_space(20.0);

                // --- Botão Direito ---
                let n_cap = self.total_capitulos_do_livro(self.livro_selecionado);
                let btn_next = egui::Button::new(
                    egui::RichText::new(">")
                        .size(24.0)
                        .color(egui::Color32::from_rgb(138, 154, 91)),
                )
                .frame(false);
                if ui.add_enabled(self.capitulo < n_cap, btn_next).clicked() {
                    self.proximo_livro();
                    self.capitulo_mudou = true;
                }
            });
        });

        ui.separator();

        let altura_do_texto = 24.0; // Estimativa da altura de cada linha
        let total_versiculos = self.versiculos.len();

        let mut scroll_area = egui::ScrollArea::vertical();

        // Reseta o scroll se o capítulo mudou
        if self.capitulo_mudou {
            scroll_area = scroll_area.scroll_offset(egui::Vec2::ZERO);
            self.capitulo_mudou = false;
        }

        scroll_area.show_rows(ui, altura_do_texto, total_versiculos, |ui, range| {
            for v in &self.versiculos[range] {
                ui.label(format!("{}{}", v.numero_formatado, v.texto));
            }
        });
    }

    fn ui_busca(&mut self, ui: &mut egui::Ui) {
        ui.heading("Pesquisar na Bíblia");
        if ui.button("Tela leitura").clicked() {
            self.navegar_para(Tela::Leitura);
        }
    }

    fn ui_config(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tela de Configurações");
        if ui.button("Tela leitura").clicked() {
            self.navegar_para(Tela::Leitura);
        }
    }
}

impl eframe::App for BibliaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.renderizar_header(ui);
        // ui.ctx().set_debug_on_hover(true); // -> Pra debugar layout

        // 1. MENU LATERAL (DESKTOP & Android)
        //#[cfg(not(target_os = "android"))]
        if self.menu_aberto {
            self.renderizar_menu(ui);
        }

        if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
            println!("Botão Voltar/ESC pressionado!");
            println!("Menu aberto: {}", self.menu_aberto);
            println!("Itens no histórico: {}", self.historico.len());

            if self.menu_aberto {
                self.menu_aberto = false;
            } else if !self.historico.is_empty() {
                self.voltar();
            } else {
                // Estamos na tela inicial. Se clicar de novo, fecha.
                if self.aguardando_saida {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                } else {
                    self.aguardando_saida = true;
                    // Aqui você poderia mostrar uma pequena mensagem (Toast)
                    println!("Pressione voltar novamente para sair");
                }
            }
        }

        // 2. MENU INFERIOR (ANDROID)
        // #[cfg(target_os = "android")]
        // egui::Panel::bottom("navegacao_mobile").show_inside(ui, |ui| {
        //     ui.horizontal(|ui| {
        //         egui::ComboBox::from_label("")
        //             .selected_text(&self.nome_livro)
        //             .show_ui(ui, |ui| {
        //                 for livro in &self.lista_livros.clone() {
        //                     if ui
        //                         .selectable_value(
        //                             &mut self.livro_selecionado,
        //                             livro.id,
        //                             &livro.name,
        //                         )
        //                         .clicked()
        //                     {
        //                         self.nome_livro = livro.name.clone();
        //                         self.capitulo = 1;
        //                         self.carregar_capitulo();
        //                     }
        //                 }
        //             });
        //         if ui
        //             .add_enabled(self.capitulo > 1, egui::Button::new("⬅ Anterior"))
        //             .clicked()
        //         {
        //             self.livro_anterior();
        //         }
        //         let n_capitulos = self.total_capitulos_do_livro(self.livro_selecionado);

        //         if ui
        //             .add_enabled(self.capitulo < n_capitulos, egui::Button::new("Próximo ➡"))
        //             .clicked()
        //         {
        //             self.proximo_livro();
        //         }
        //     });
        // });

        let mut frame = egui::Frame::central_panel(&ui.ctx().global_style());
        frame.inner_margin.top = 40;

        // 3. ÁREA DE LEITURA
        egui::CentralPanel::default()
            .frame(frame)
            .show_inside(ui, |ui| {
                match self.tela_atual {
                    Tela::Leitura => self.ui_leitura(ui),
                    Tela::Busca => self.ui_busca(ui),
                    Tela::Configuracoes => self.ui_config(ui),
                    // _ => ui.label("Nenhuma?"),
                }
            });
    }
}
