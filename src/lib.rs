use std::sync::mpsc::channel;

use eframe::{
    App,
    egui::{self, FontId, RichText, TextStyle, Visuals},
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
    pub cor_hex: Option<String>,
    pub favorito: bool,
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
    selecionado: Option<i32>,
    termo_busca: String,
    resultados: Vec<ResultadoBusca>,
    pular_para_versiculo: Option<i32>,
    buscando: bool,
    tx_busca: std::sync::mpsc::Sender<Vec<ResultadoBusca>>,
    rx_busca: std::sync::mpsc::Receiver<Vec<ResultadoBusca>>,
    tema_escuro: bool,
    tamanho_fonte: f32,
}

pub struct ResultadoBusca {
    pub livro_nome: String,
    pub livro_id: i32,
    pub capitulo: i32,
    pub numero: i32,
    pub texto: String,
}

impl BibliaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configura_context(&cc.egui_ctx);

        let (tx, rx) = channel();

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
            selecionado: None,
            termo_busca: String::new(),
            resultados: Vec::new(),
            pular_para_versiculo: None,
            buscando: false,
            tx_busca: tx,
            rx_busca: rx,
            tema_escuro: false,
            tamanho_fonte: 20.0,
        };

        app.inicializar_banco();
        app.carregar_lista_livros();
        app.carregar_capitulo();

        app
    }

    pub fn inicializar_banco(&mut self) {
        let path = crate::db::get_db_path();

        if let Ok(conn) = Connection::open(path) {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS marcacoes (
                    book INTEGER,
                    chapter INTEGER,
                    verse INTEGER,
                    cor TEXT,
                    favorito INTEGER DEFAULT 0,
                    PRIMARY KEY (book, chapter, verse)
                )",
                [],
            )
            .ok();

            // Dica: Adicione um índice para buscas rápidas por cor/favorito no futuro
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_favoritos ON marcacoes(favorito)",
                [],
            )
            .ok();

            if let Err(e) = crate::db::otimizar_banco_se_necessario(&conn) {
                eprintln!("Erro na otimização: {}", e);
            }
        }
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
        #[cfg(target_os = "android")]
        {
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            style.spacing.button_padding = egui::vec2(10.0, 8.0);

            style.text_styles.insert(
                TextStyle::Body,
                FontId::new(18.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                TextStyle::Heading,
                FontId::new(24.0, egui::FontFamily::Proportional),
            );

            style.visuals.window_corner_radius = 12.0.into();
            style.visuals.widgets.noninteractive.corner_radius = 8.0.into();
        }
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
                    .prepare("SELECT v.verse, v.text, m.cor, m.favorito
                                 FROM verses v
                                 LEFT JOIN marcacoes m ON v.book = m.book AND v.chapter = m.chapter AND v.verse = m.verse
                                 WHERE v.book = ?1 AND v.chapter = ?2")
                    .unwrap();

                let iter = stmt
                    .query_map([self.livro_selecionado, self.capitulo], |row| {
                        let num: i32 = row.get(0)?;
                        let fav_int: Option<i32> = row.get(3)?;

                        Ok(Versiculo {
                            numero: num,
                            numero_formatado: self.formatar_elevado(&num),
                            texto: row.get(1)?,
                            cor_hex: row.get(2).ok(), // Se for NULL, vira None
                            favorito: fav_int.unwrap_or(0) == 1,
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

    fn executar_busca_async(&mut self) {
        let termo_original = self.termo_busca.trim().to_string();
        if termo_original.is_empty() {
            return;
        }

        // 1. Marcamos que estamos buscando para a UI mostrar o Spinner
        self.buscando = true;

        // 2. Clonamos o que a thread vai precisar
        let tx = self.tx_busca.clone();
        let termo_limpo = normalizar(&termo_original);

        // 3. Criamos a thread (A mágica acontece aqui)
        std::thread::spawn(move || {
            let path = crate::db::get_db_path();
            let mut resultados = Vec::new();

            if let Ok(conn) = rusqlite::Connection::open(path) {
                // USANDO O ÍNDICE: v.texto_busca é MUITO mais rápido que o LIKE '%text%'
                let mut stmt = conn
                    .prepare(
                        "SELECT b.name, v.book, v.chapter, v.verse, v.text
                     FROM verses v
                     JOIN books b ON v.book = b.id
                     WHERE v.texto_busca LIKE ?1
                     LIMIT 200",
                    )
                    .unwrap();

                // Usamos a busca por frase/palavra no SQL
                let termo_sql = format!("%{}%", termo_limpo);

                if let Ok(iter) = stmt.query_map([termo_sql], |row| {
                    Ok(ResultadoBusca {
                        livro_nome: row.get(0)?,
                        livro_id: row.get(1)?,
                        capitulo: row.get(2)?,
                        numero: row.get(3)?,
                        texto: row.get(4)?,
                    })
                }) {
                    // Refinamos no Rust para garantir que é a palavra exata (evitar o todavia)
                    resultados = iter
                        .filter_map(|res| res.ok())
                        .filter(|res| {
                            normalizar(&res.texto)
                                .split_whitespace()
                                .any(|p| p == termo_limpo)
                        })
                        .collect();
                }
            }

            // 4. Enviamos os resultados de volta para o canal (rx_busca vai receber)
            let _ = tx.send(resultados);
        });
    }

    fn aplicar_tema_e_fonte(&self, ctx: &egui::Context) {
        // 1. Aplica o Tema Claro/Escuro
        let visuals = if self.tema_escuro {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        ctx.set_visuals(visuals);

        // 2. Aplica o Tamanho da Fonte Dinamicamente
        let mut style = (*ctx.style()).clone();

        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(self.tamanho_fonte, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(self.tamanho_fonte + 6.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(self.tamanho_fonte - 2.0, egui::FontFamily::Proportional),
        );

        // Espaçamentos amigáveis para toque (Mobile/Desktop)
        style.spacing.item_spacing = egui::vec2(12.0, 12.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);

        ctx.set_style(style);
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

    fn salvar_marcacao(&mut self, num_v: i32, cor: Option<&str>, fav: Option<bool>) {
        let path = crate::db::get_db_path();
        if let Ok(conn) = Connection::open(path) {
            // Usamos INSERT OR IGNORE para garantir que a linha exista
            conn.execute(
                "INSERT OR IGNORE INTO marcacoes (book, chapter, verse, favorito) VALUES (?1, ?2, ?3, 0)",
                rusqlite::params![self.livro_selecionado, self.capitulo, num_v],
            ).ok();

            if let Some(c) = cor {
                conn.execute(
                    "UPDATE marcacoes SET cor = ?1 WHERE book = ?2 AND chapter = ?3 AND verse = ?4",
                    rusqlite::params![c, self.livro_selecionado, self.capitulo, num_v],
                )
                .ok();
            }

            if let Some(f) = fav {
                let val = if f { 1 } else { 0 };
                conn.execute(
                    "UPDATE marcacoes SET favorito = ?1 WHERE book = ?2 AND chapter = ?3 AND verse = ?4",
                    rusqlite::params![val, self.livro_selecionado, self.capitulo, num_v],
                ).ok();
            }
        }
        self.carregar_capitulo(); // Recarrega a UI
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
                        // Ícone da engrenagem para config
                        if ui.button(egui::RichText::new("⚙").size(20.0)).clicked() {
                            self.navegar_para(Tela::Configuracoes);
                        }
                        // Ícone de lupa para busca
                        if ui.button(egui::RichText::new("🔍").size(20.0)).clicked() {
                            self.navegar_para(Tela::Busca);
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
            // for v in &self.versiculos[range] {
            //     ui.label(format!("{}{}", v.numero_formatado, v.texto));
            // }
            let mut acao_para_salvar = None;
            let mut acao_para_limpar = None;
            let mut novo_selecionado = self.selecionado;

            for v in &self.versiculos {
                let mut texto_rt =
                    egui::RichText::new(format!("{} {}", v.numero_formatado, v.texto));

                if let Some(hex) = &v.cor_hex {
                    texto_rt = texto_rt.background_color(hex_para_color32(hex));
                }

                if v.favorito {
                    texto_rt = texto_rt.color(egui::Color32::GOLD).strong();
                }

                let is_selected = self.selecionado == Some(v.numero);

                // RENDERIZA O VERSÍCULO
                let resp = ui.selectable_label(is_selected, texto_rt);

                // LÓGICA DE SCROLL: Se viemos da busca, pula para cá
                if self.pular_para_versiculo == Some(v.numero) {
                    resp.scroll_to_me(Some(egui::Align::TOP));
                    novo_selecionado = Some(v.numero); // Destaca ele
                    self.pular_para_versiculo = None; // Limpa a flag
                }

                if resp.clicked() {
                    novo_selecionado = if is_selected { None } else { Some(v.numero) };
                }

                // MENU DE CORES
                if is_selected {
                    ui.horizontal(|ui| {
                        if ui.button("🟡 Amarelo").clicked() {
                            acao_para_salvar = Some((v.numero, Some("#FFFF00"), None));
                        }
                        if ui.button("⭐ Favorito").clicked() {
                            acao_para_salvar = Some((v.numero, None, Some(true)));
                        }
                        if ui.button("🗑️ Limpar").clicked() {
                            acao_para_limpar = Some(v.numero);
                        }
                    });
                }
            }

            // APLICA AS MUDANÇAS (Fora do loop para evitar erro de borrow)
            self.selecionado = novo_selecionado;

            if let Some((num, cor, fav)) = acao_para_salvar {
                self.salvar_marcacao(num, cor, fav);
            }
            if let Some(num) = acao_para_limpar {
                self.limpar_marcacao(num);
            }
        });
    }

    fn ui_busca(&mut self, ui: &mut egui::Ui) {
        // 1. CHECAGEM ASSÍNCRONA: Verifica se a thread de busca enviou novos resultados
        if let Ok(novos_resultados) = self.rx_busca.try_recv() {
            self.resultados = novos_resultados;
            self.buscando = false;
        }

        ui.vertical(|ui| {
            ui.heading("Pesquisar na Bíblia");

            ui.horizontal(|ui| {
                // Campo de texto
                let edit = ui.text_edit_singleline(&mut self.termo_busca);

                // Dispara a busca ao apertar Enter ou clicar na lupa
                let enter_pressionado =
                    edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                if enter_pressionado || ui.button("🔍").clicked() {
                    // Chamamos a função que dispara a THREAD (não trava a UI)
                    self.executar_busca_async();
                }

                // Exibe um spinner (carregamento) enquanto a thread trabalha
                if self.buscando {
                    ui.add(egui::Spinner::new().size(16.0));
                }
            });

            ui.separator();

            let mut destino_clique = None;

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2]) // Melhora o comportamento no Android
                .show(ui, |ui| {
                    if self.resultados.is_empty() && !self.buscando {
                        ui.label("Nenhum resultado encontrado.");
                    }

                    // Armazenamos o termo normalizado para o realce (highlight)
                    let termo_norm = normalizar(&self.termo_busca);

                    for res in &self.resultados {
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                // Título: "Gênesis 1:1"
                                let titulo =
                                    format!("{} {}:{}", res.livro_nome, res.capitulo, res.numero);
                                if ui.link(egui::RichText::new(titulo).strong()).clicked() {
                                    destino_clique = Some((res.livro_id, res.capitulo, res.numero));
                                }

                                // Exibe o texto com realce inteligente
                                ui.horizontal_wrapped(|ui| {
                                    for palavra in res.texto.split_whitespace() {
                                        // Se a palavra normalizada for igual ao termo normalizado
                                        if normalizar(palavra) == termo_norm {
                                            ui.label(
                                                egui::RichText::new(palavra)
                                                    .color(egui::Color32::GOLD)
                                                    .strong(),
                                            );
                                        } else {
                                            ui.label(palavra);
                                        }
                                    }
                                });
                            });
                        });
                        ui.add_space(4.0);
                    }

                    // Lógica de navegação ao clicar em um resultado
                    if let Some((livro_id, cap_num, versiculo_num)) = destino_clique {
                        self.livro_selecionado = livro_id;
                        self.capitulo = cap_num;
                        self.pular_para_versiculo = Some(versiculo_num);
                        self.carregar_capitulo();
                        self.navegar_para(Tela::Leitura);
                    }
                });
        });
    }

    fn ui_config(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("⚙ Configurações");
        });
        ui.separator();
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Seção de Aparência
            ui.label(
                egui::RichText::new("Aparência")
                    .strong()
                    .color(ui.visuals().warn_fg_color),
            );
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tema_escuro, false, "🌞 Claro");
                ui.selectable_value(&mut self.tema_escuro, true, "🌙 Escuro");
            });

            ui.add_space(20.0);

            // Seção de Leitura (Fonte)
            ui.label(
                egui::RichText::new("Tamanho do Texto")
                    .strong()
                    .color(ui.visuals().warn_fg_color),
            );
            ui.horizontal(|ui| {
                if ui.button(" A- ").clicked() {
                    self.tamanho_fonte = (self.tamanho_fonte - 2.0).max(12.0);
                }
                ui.add(egui::Slider::new(&mut self.tamanho_fonte, 12.0..=36.0).text("px"));
                if ui.button(" A+ ").clicked() {
                    self.tamanho_fonte = (self.tamanho_fonte + 2.0).min(36.0);
                }
            });

            ui.add_space(20.0);

            // Preview da Leitura
            ui.group(|ui| {
                ui.label(egui::RichText::new("Pré-visualização:").italics());
                ui.add_space(5.0);
                ui.label(format!(
                    "{} No princípio, criou Deus os céus e a terra.",
                    self.formatar_elevado(&1)
                ));
            });

            ui.add_space(30.0);

            // Botão de voltar grande
            ui.vertical_centered(|ui| {
                if ui
                    .add(
                        egui::Button::new("Voltar para Leitura")
                            .fill(egui::Color32::from_rgb(138, 154, 91)),
                    )
                    .clicked()
                {
                    self.navegar_para(Tela::Leitura);
                }
            });
        });
    }

    fn limpar_marcacao(&mut self, num_v: i32) {
        let path = crate::db::get_db_path();
        if let Ok(conn) = rusqlite::Connection::open(path) {
            // Remove a linha da tabela de marcações
            conn.execute(
                "DELETE FROM marcacoes WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                rusqlite::params![self.livro_selecionado, self.capitulo, num_v],
            )
            .ok();
        }

        // Atualiza a lista da memória e limpa a seleção da tela
        self.selecionado = None;
        self.carregar_capitulo();
    }
}

impl eframe::App for BibliaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.aplicar_tema_e_fonte(ui.ctx());

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
                if self.aguardando_saida {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                } else {
                    self.aguardando_saida = true;
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
                self.renderizar_header(ui);

                match self.tela_atual {
                    Tela::Leitura => self.ui_leitura(ui),
                    Tela::Busca => self.ui_busca(ui),
                    Tela::Configuracoes => self.ui_config(ui),
                    // _ => ui.label("Nenhuma?"),
                }
            });
    }
}

fn hex_para_color32(hex: &str) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');

    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
        egui::Color32::from_rgb(r, g, b)
    } else {
        egui::Color32::from_rgb(255, 255, 0)
    }
}

fn normalizar(texto: &str) -> String {
    use unicode_normalization::UnicodeNormalization;

    texto
        .nfd()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}
