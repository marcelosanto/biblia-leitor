use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, RichText, Sense, Stroke, Ui, pos2,
    text::{LayoutJob, TextFormat},
    vec2,
};
use rusqlite::Connection;

mod db;
mod tema;

use tema::{
    Icone, Paleta, Tema, aba, alpha, botao_icone, chip, cor_destaque, desenhar_icone, hex_cor,
    serif,
};

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

// Áreas seguras (barra de status / gestos) no Android.
const SAFE_TOP: i8 = if cfg!(target_os = "android") { 32 } else { 0 };
const SAFE_BOTTOM: i8 = if cfg!(target_os = "android") { 14 } else { 0 };
const LARGURA_LEITURA: f32 = 680.0;

/// Mesmos hex do app antigo: marcações já salvas continuam válidas.
const MARCAS: [(&str, Color32); 4] = [
    ("#FFF83B", Color32::from_rgb(255, 224, 80)),
    ("#90EE90", Color32::from_rgb(120, 210, 130)),
    ("#ADD8E6", Color32::from_rgb(120, 180, 225)),
    ("#FFB6C1", Color32::from_rgb(240, 140, 165)),
];

#[derive(PartialEq, Clone, Copy)]
enum Tela {
    Leitura,
    Livros,
    Busca,
    Salvos,
}

#[derive(PartialEq, Clone, Copy)]
enum FiltroSalvos {
    Favoritos,
    Destaques,
}

enum Acao {
    Cor(&'static str),
    LimparCor,
    Copiar,
    Favorito,
}

#[derive(Clone)]
pub struct Livro {
    pub id: i32,
    pub name: String,
    pub abbrev: String,
}

pub struct Versiculo {
    pub numero: i32,
    pub texto: String,
    pub cor: Option<String>,
    pub favorito: bool,
}

pub struct ResultadoBusca {
    pub livro_nome: String,
    pub livro_id: i32,
    pub capitulo: i32,
    pub numero: i32,
    pub texto: String,
}

pub struct Salvo {
    pub livro_id: i32,
    pub livro_nome: String,
    pub capitulo: i32,
    pub numero: i32,
    pub texto: String,
    pub cor: Option<String>,
    pub favorito: bool,
}

pub struct BibliaApp {
    tela: Tela,
    tema: Tema,
    tamanho_fonte: f32,
    estilo_sujo: bool,

    // leitura
    id_livro: i32,
    nome_livro: String,
    capitulo: i32,
    total_caps: i32,
    lista_livros: Vec<Livro>,
    versiculos: Vec<Versiculo>,
    selecionado: Option<i32>,
    pular_para: Option<i32>,
    rolar_topo: bool,

    // livros
    livro_em_escolha: Option<Livro>,
    caps_escolha: i32,
    filtro_livro: String,

    // busca
    termo_busca: String,
    resultados: Vec<ResultadoBusca>,
    buscando: bool,
    busca_feita: bool,
    focar_busca: bool,
    tx_busca: Sender<Vec<ResultadoBusca>>,
    rx_busca: Receiver<Vec<ResultadoBusca>>,

    // salvos
    salvos: Vec<Salvo>,
    filtro_salvos: FiltroSalvos,

    // ajustes / navegação
    ajustes_abertos: bool,
    rect_aa: egui::Rect,
    aguardando_saida: bool,
}

impl BibliaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        tema::carregar_fontes(&cc.egui_ctx);

        let (tx, rx) = channel();
        let mut app = Self {
            tela: Tela::Leitura,
            tema: Tema::Claro,
            tamanho_fonte: 20.0,
            estilo_sujo: true,
            id_livro: 1,
            nome_livro: "Gênesis".to_string(),
            capitulo: 1,
            total_caps: 1,
            lista_livros: Vec::new(),
            versiculos: Vec::new(),
            selecionado: None,
            pular_para: None,
            rolar_topo: true,
            livro_em_escolha: None,
            caps_escolha: 1,
            filtro_livro: String::new(),
            termo_busca: String::new(),
            resultados: Vec::new(),
            buscando: false,
            busca_feita: false,
            focar_busca: false,
            tx_busca: tx,
            rx_busca: rx,
            salvos: Vec::new(),
            filtro_salvos: FiltroSalvos::Favoritos,
            ajustes_abertos: false,
            rect_aa: egui::Rect::NOTHING,
            aguardando_saida: false,
        };

        app.inicializar_banco();
        app.carregar_lista_livros();

        app.tema = Tema::de_chave(&app.ler_config("tema", "claro"));
        app.tamanho_fonte = app
            .ler_config("tamanho_fonte", "20")
            .parse()
            .unwrap_or(20.0);
        tema::aplicar(&cc.egui_ctx, app.tema);
        app.estilo_sujo = false;

        // Continua de onde parou
        let posicao = app.ler_config("posicao", "1:1");
        let mut partes = posicao.split(':');
        let id = partes.next().and_then(|s| s.parse().ok()).unwrap_or(1);
        let cap = partes.next().and_then(|s| s.parse().ok()).unwrap_or(1);
        app.ir_para(id, cap, None);

        app
    }

    // ───────────────────────── Banco de dados ─────────────────────────

    pub fn inicializar_banco(&mut self) {
        let path = crate::db::get_db_path();
        if let Ok(conn) = Connection::open(path) {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS marcacoes (
                    book INTEGER, chapter INTEGER, verse INTEGER, cor TEXT,
                    favorito INTEGER DEFAULT 0,
                    PRIMARY KEY (book, chapter, verse)
                )",
                [],
            )
            .ok();
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_favoritos ON marcacoes(favorito)",
                [],
            )
            .ok();
            conn.execute(
                "CREATE TABLE IF NOT EXISTS config (chave TEXT PRIMARY KEY, valor TEXT)",
                [],
            )
            .ok();
            if let Err(e) = crate::db::otimizar_banco_se_necessario(&conn) {
                eprintln!("Erro na otimização: {}", e);
            }
        }
    }

    fn abrir(&self) -> Option<Connection> {
        Connection::open(crate::db::get_db_path()).ok()
    }

    fn carregar_lista_livros(&mut self) {
        let Some(conn) = self.abrir() else { return };
        let Ok(mut stmt) = conn.prepare("SELECT id, name, abbrev FROM books ORDER BY id") else {
            return;
        };
        if let Ok(iter) = stmt.query_map([], |row| {
            Ok(Livro {
                id: row.get(0)?,
                name: row.get(1)?,
                abbrev: row.get(2)?,
            })
        }) {
            self.lista_livros = iter.filter_map(|r| r.ok()).collect();
        }
    }

    fn consultar_total_caps(&self, livro_id: i32) -> i32 {
        self.abrir()
            .and_then(|conn| {
                conn.query_row(
                    "SELECT COALESCE(MAX(chapter), 1) FROM verses WHERE book = ?1",
                    [livro_id],
                    |row| row.get::<_, i32>(0),
                )
                .ok()
            })
            .unwrap_or(1)
    }

    fn carregar_versiculos(&mut self) {
        let Some(conn) = self.abrir() else { return };
        let Ok(mut stmt) = conn.prepare(
            "SELECT v.verse, v.text, m.cor, m.favorito
             FROM verses v
             LEFT JOIN marcacoes m ON v.book = m.book AND v.chapter = m.chapter AND v.verse = m.verse
             WHERE v.book = ?1 AND v.chapter = ?2
             ORDER BY v.verse ASC",
        ) else {
            return;
        };
        if let Ok(iter) = stmt.query_map([self.id_livro, self.capitulo], |row| {
            let fav: Option<i32> = row.get(3)?;
            Ok(Versiculo {
                numero: row.get(0)?,
                texto: row.get(1)?,
                cor: row.get::<_, Option<String>>(2)?,
                favorito: fav.unwrap_or(0) == 1,
            })
        }) {
            self.versiculos = iter.filter_map(|v| v.ok()).collect();
        }
    }

    fn carregar_salvos(&mut self) {
        let filtro = match self.filtro_salvos {
            FiltroSalvos::Favoritos => "m.favorito = 1",
            FiltroSalvos::Destaques => "m.cor IS NOT NULL",
        };
        let sql = format!(
            "SELECT m.book, b.name, m.chapter, m.verse, v.text, m.cor, m.favorito
             FROM marcacoes m
             JOIN verses v ON v.book = m.book AND v.chapter = m.chapter AND v.verse = m.verse
             JOIN books b ON b.id = m.book
             WHERE {filtro}
             ORDER BY m.book, m.chapter, m.verse LIMIT 500"
        );
        self.salvos.clear();
        let Some(conn) = self.abrir() else { return };
        let Ok(mut stmt) = conn.prepare(&sql) else {
            return;
        };
        if let Ok(iter) = stmt.query_map([], |row| {
            let fav: Option<i32> = row.get(6)?;
            Ok(Salvo {
                livro_id: row.get(0)?,
                livro_nome: row.get(1)?,
                capitulo: row.get(2)?,
                numero: row.get(3)?,
                texto: row.get(4)?,
                cor: row.get::<_, Option<String>>(5)?,
                favorito: fav.unwrap_or(0) == 1,
            })
        }) {
            self.salvos = iter.filter_map(|r| r.ok()).collect();
        }
    }

    fn executar_busca_async(&mut self) {
        // Remove aspas do usuário e busca a frase exata no FTS5.
        let termo = self.termo_busca.replace('"', " ").trim().to_string();
        if termo.is_empty() {
            return;
        }
        self.buscando = true;
        let tx = self.tx_busca.clone();

        std::thread::spawn(move || {
            let mut resultados = Vec::new();
            if let Ok(conn) = Connection::open(crate::db::get_db_path()) {
                if let Ok(mut stmt) = conn.prepare(
                    "SELECT b.name, f.book, f.chapter, f.verse, f.text
                     FROM verses_fts f
                     JOIN books b ON f.book = b.id
                     WHERE f.text MATCH ?1
                     ORDER BY rank
                     LIMIT 50",
                ) {
                    let termo_match = format!("\"{}\"", termo);
                    if let Ok(iter) = stmt.query_map([termo_match], |row| {
                        Ok(ResultadoBusca {
                            livro_nome: row.get(0)?,
                            livro_id: row.get(1)?,
                            capitulo: row.get(2)?,
                            numero: row.get(3)?,
                            texto: row.get(4)?,
                        })
                    }) {
                        resultados = iter.filter_map(|r| r.ok()).collect();
                    }
                }
            }
            let _ = tx.send(resultados);
        });
    }

    fn salvar_config(&self, chave: &str, valor: &str) {
        if let Some(conn) = self.abrir() {
            conn.execute(
                "INSERT OR REPLACE INTO config (chave, valor) VALUES (?1, ?2)",
                [chave, valor],
            )
            .ok();
        }
    }

    fn ler_config(&self, chave: &str, padrao: &str) -> String {
        self.abrir()
            .and_then(|conn| {
                conn.query_row("SELECT valor FROM config WHERE chave = ?1", [chave], |r| {
                    r.get::<_, String>(0)
                })
                .ok()
            })
            .unwrap_or_else(|| padrao.to_string())
    }

    fn salvar_marcacao(&mut self, num_v: i32, cor: Option<&str>, fav: Option<bool>) {
        if let Some(conn) = self.abrir() {
            conn.execute(
                "INSERT OR IGNORE INTO marcacoes (book, chapter, verse, favorito) VALUES (?1, ?2, ?3, 0)",
                rusqlite::params![self.id_livro, self.capitulo, num_v],
            )
            .ok();
            if let Some(c) = cor {
                conn.execute(
                    "UPDATE marcacoes SET cor = ?1 WHERE book = ?2 AND chapter = ?3 AND verse = ?4",
                    rusqlite::params![c, self.id_livro, self.capitulo, num_v],
                )
                .ok();
            }
            if let Some(f) = fav {
                conn.execute(
                    "UPDATE marcacoes SET favorito = ?1 WHERE book = ?2 AND chapter = ?3 AND verse = ?4",
                    rusqlite::params![f as i32, self.id_livro, self.capitulo, num_v],
                )
                .ok();
            }
        }
        self.carregar_versiculos();
    }

    /// Remove só a cor; se não sobrou nada (nem favorito), apaga a linha.
    fn limpar_cor(&mut self, num_v: i32) {
        if let Some(conn) = self.abrir() {
            let p = rusqlite::params![self.id_livro, self.capitulo, num_v];
            conn.execute(
                "UPDATE marcacoes SET cor = NULL WHERE book = ?1 AND chapter = ?2 AND verse = ?3",
                p,
            )
            .ok();
            conn.execute(
                "DELETE FROM marcacoes WHERE cor IS NULL AND favorito = 0
                 AND book = ?1 AND chapter = ?2 AND verse = ?3",
                p,
            )
            .ok();
        }
        self.carregar_versiculos();
    }

    // ───────────────────────── Navegação ─────────────────────────

    fn ir_para(&mut self, id: i32, cap: i32, pular: Option<i32>) {
        if let Some(l) = self.lista_livros.iter().find(|l| l.id == id) {
            self.nome_livro = l.name.clone();
        }
        self.id_livro = id;
        self.capitulo = cap;
        self.total_caps = self.consultar_total_caps(id); // cacheado: nada de SQL por frame
        self.selecionado = None;
        self.pular_para = pular;
        self.rolar_topo = pular.is_none();
        self.livro_em_escolha = None;
        self.tela = Tela::Leitura;
        self.carregar_versiculos();
        self.salvar_config("posicao", &format!("{id}:{cap}"));
    }

    /// Avança/volta capítulo; nas pontas, passa para o livro vizinho.
    fn mudar_capitulo(&mut self, delta: i32) {
        let novo = self.capitulo + delta;
        if novo >= 1 && novo <= self.total_caps {
            self.ir_para(self.id_livro, novo, None);
            return;
        }
        let Some(i) = self.lista_livros.iter().position(|l| l.id == self.id_livro) else {
            return;
        };
        if delta > 0 && i + 1 < self.lista_livros.len() {
            let id = self.lista_livros[i + 1].id;
            self.ir_para(id, 1, None);
        } else if delta < 0 && i > 0 {
            let id = self.lista_livros[i - 1].id;
            let total = self.consultar_total_caps(id);
            self.ir_para(id, total, None);
        }
    }

    fn mudar_tela(&mut self, tela: Tela) {
        self.selecionado = None;
        self.livro_em_escolha = None;
        self.ajustes_abertos = false;
        if tela == Tela::Salvos {
            self.carregar_salvos();
        }
        if tela == Tela::Busca {
            self.focar_busca = true;
        }
        self.tela = tela;
    }

    fn tratar_voltar(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.pointer.any_pressed()) {
            self.aguardando_saida = false;
        }
        if !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return;
        }
        if self.ajustes_abertos {
            self.ajustes_abertos = false;
        } else if self.selecionado.is_some() {
            self.selecionado = None;
        } else if self.livro_em_escolha.is_some() {
            self.livro_em_escolha = None;
        } else if self.tela != Tela::Leitura {
            self.tela = Tela::Leitura;
        } else if self.aguardando_saida {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            self.aguardando_saida = true;
        }
    }

    // ───────────────────────── Barras ─────────────────────────

    fn barra_topo(&mut self, ui: &mut Ui, p: &Paleta) {
        let frame = Frame::NONE.fill(p.fundo).inner_margin(Margin {
            left: 16,
            right: 8,
            top: SAFE_TOP + 8,
            bottom: 6,
        });
        let mut abrir_livros = false;
        let mut delta = 0;
        let mut alternar_ajustes = false;
        let mut rect_aa = self.rect_aa;

        egui::Panel::top("topo").frame(frame).show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.tela == Tela::Leitura {
                    let titulo = format!("{} {}", self.nome_livro, self.capitulo);
                    if pilula(ui, &titulo, p).clicked() {
                        abrir_livros = true;
                    }
                } else {
                    let t = match self.tela {
                        Tela::Livros => "Livros",
                        Tela::Busca => "Buscar",
                        _ => "Salvos",
                    };
                    ui.label(RichText::new(t).font(FontId::new(26.0, serif())).strong());
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let r = botao_icone(ui, Icone::Aa, p.texto);
                    rect_aa = r.rect;
                    if r.clicked() {
                        alternar_ajustes = true;
                    }
                    if self.tela == Tela::Leitura {
                        if botao_icone(ui, Icone::Dir, p.texto).clicked() {
                            delta = 1;
                        }
                        if botao_icone(ui, Icone::Esq, p.texto).clicked() {
                            delta = -1;
                        }
                    }
                });
            });
        });

        self.rect_aa = rect_aa;
        if abrir_livros {
            self.mudar_tela(Tela::Livros);
            self.caps_escolha = self.total_caps;
            self.livro_em_escolha = self
                .lista_livros
                .iter()
                .find(|l| l.id == self.id_livro)
                .cloned();
        }
        if alternar_ajustes {
            self.ajustes_abertos = !self.ajustes_abertos;
        }
        if delta != 0 {
            self.mudar_capitulo(delta);
        }
    }

    fn barra_inferior(&mut self, ui: &mut Ui, p: &Paleta) {
        let frame = Frame::NONE
            .fill(p.superficie)
            .stroke(Stroke::new(1.0, p.borda))
            .inner_margin(Margin {
                left: 8,
                right: 8,
                top: 4,
                bottom: SAFE_BOTTOM + 2,
            });
        let mut destino = None;
        let atual = self.tela;

        egui::Panel::bottom("nav").frame(frame).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.columns(4, |c| {
                if aba(&mut c[0], Icone::Ler, "Ler", atual == Tela::Leitura, p).clicked() {
                    destino = Some(Tela::Leitura);
                }
                if aba(&mut c[1], Icone::Livros, "Livros", atual == Tela::Livros, p).clicked() {
                    destino = Some(Tela::Livros);
                }
                if aba(&mut c[2], Icone::Busca, "Buscar", atual == Tela::Busca, p).clicked() {
                    destino = Some(Tela::Busca);
                }
                if aba(&mut c[3], Icone::Salvos, "Salvos", atual == Tela::Salvos, p).clicked() {
                    destino = Some(Tela::Salvos);
                }
            });
        });

        if let Some(t) = destino {
            self.mudar_tela(t);
        }
    }

    // ───────────────────────── Leitura ─────────────────────────

    fn ui_leitura(&mut self, ui: &mut Ui, p: &Paleta) {
        let area_central = ui.max_rect();

        if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.mudar_capitulo(-1);
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.mudar_capitulo(1);
        }

        let tam = self.tamanho_fonte;
        let mut novo_sel = self.selecionado;
        let mut pular = self.pular_para;
        let mut nav = 0;

        let mut area = egui::ScrollArea::vertical().auto_shrink([false; 2]);
        if self.rolar_topo {
            area = area.scroll_offset(egui::Vec2::ZERO);
            self.rolar_topo = false;
        }

        area.show(ui, |ui| {
            coluna(ui, LARGURA_LEITURA, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                // Cabeçalho do capítulo
                ui.add_space(20.0);
                ui.label(
                    RichText::new(self.nome_livro.to_uppercase())
                        .size(13.0)
                        .strong()
                        .extra_letter_spacing(2.0)
                        .color(p.acento),
                );
                ui.label(
                    RichText::new(format!("Capítulo {}", self.capitulo))
                        .font(FontId::new(36.0, serif()))
                        .color(p.texto),
                );
                ui.add_space(10.0);

                for v in &self.versiculos {
                    let sel = novo_sel == Some(v.numero);

                    // reserva a camada do fundo de seleção (desenhada atrás do texto)
                    let fundo_idx = ui.painter().add(egui::Shape::Noop);
                    let resp =
                        ui.add(egui::Label::new(job_versiculo(v, p, tam)).sense(Sense::click()));
                    if sel {
                        ui.painter().set(
                            fundo_idx,
                            egui::Shape::rect_filled(
                                resp.rect.expand2(vec2(10.0, 4.0)),
                                10.0,
                                alpha(p.acento, 26),
                            ),
                        );
                    }

                    if resp.clicked() {
                        novo_sel = if sel { None } else { Some(v.numero) };
                    }
                    if pular == Some(v.numero) {
                        resp.scroll_to_me(Some(Align::Center));
                        novo_sel = Some(v.numero);
                        pular = None;
                    }
                }

                // Navegação no fim do capítulo
                ui.add_space(28.0);
                ui.horizontal(|ui| {
                    if chip(ui, "‹  Anterior", false, p, vec2(120.0, 44.0)).clicked() {
                        nav = -1;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if chip(ui, "Próximo  ›", true, p, vec2(120.0, 44.0)).clicked() {
                            nav = 1;
                        }
                    });
                });
                ui.add_space(140.0); // folga para a barra de ações
            });
        });

        self.selecionado = novo_sel;
        self.pular_para = pular;
        if nav != 0 {
            self.mudar_capitulo(nav);
        }

        let ancora = pos2(area_central.center().x, area_central.bottom() - 16.0);
        self.barra_acoes(ui.ctx(), p, ancora);
    }

    fn barra_acoes(&mut self, ctx: &egui::Context, p: &Paleta, ancora: egui::Pos2) {
        let Some(sel) = self.selecionado else { return };
        let (cor_atual, fav, texto) = self
            .versiculos
            .iter()
            .find(|v| v.numero == sel)
            .map(|v| (v.cor.clone(), v.favorito, v.texto.clone()))
            .unwrap_or((None, false, String::new()));

        let mut acao = None;

        egui::Area::new(egui::Id::new("barra_acoes"))
            .order(egui::Order::Foreground)
            .pivot(Align2::CENTER_BOTTOM)
            .fixed_pos(ancora)
            .show(ctx, |ui| {
                Frame::NONE
                    .fill(p.superficie)
                    .corner_radius(30.0)
                    .stroke(Stroke::new(1.0, p.borda))
                    .shadow(tema::sombra(p))
                    .inner_margin(Margin::symmetric(12, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;

                            for (hex, cor) in MARCAS {
                                let (rect, r) =
                                    ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
                                ui.painter().circle_filled(rect.center(), 12.0, cor);
                                if cor_atual.as_deref() == Some(hex) {
                                    ui.painter().circle_stroke(
                                        rect.center(),
                                        15.0,
                                        Stroke::new(2.0, p.texto),
                                    );
                                }
                                if r.clicked() {
                                    acao = Some(Acao::Cor(hex));
                                }
                            }

                            if botao_icone(ui, Icone::Limpar, p.suave).clicked() {
                                acao = Some(Acao::LimparCor);
                            }

                            let (r, _) = ui.allocate_exact_size(vec2(1.0, 24.0), Sense::hover());
                            ui.painter().rect_filled(r, 0.0, p.borda);

                            if botao_icone(ui, Icone::Copiar, p.texto).clicked() {
                                acao = Some(Acao::Copiar);
                            }
                            let (ic, cor) = if fav {
                                (Icone::EstrelaCheia, Color32::from_rgb(222, 164, 30))
                            } else {
                                (Icone::Estrela, p.texto)
                            };
                            if botao_icone(ui, ic, cor).clicked() {
                                acao = Some(Acao::Favorito);
                            }
                        });
                    });
            });

        match acao {
            Some(Acao::Cor(hex)) => self.salvar_marcacao(sel, Some(hex), None),
            Some(Acao::LimparCor) => self.limpar_cor(sel),
            Some(Acao::Favorito) => self.salvar_marcacao(sel, None, Some(!fav)),
            Some(Acao::Copiar) => {
                ctx.copy_text(format!(
                    "“{}” — {} {}:{}",
                    texto, self.nome_livro, self.capitulo, sel
                ));
                self.selecionado = None;
            }
            None => {}
        }
    }

    // ───────────────────────── Livros ─────────────────────────

    fn ui_livros(&mut self, ui: &mut Ui, p: &Paleta) {
        let mut voltar = false;
        let mut escolha: Option<Livro> = None;
        let mut ir: Option<(i32, i32)> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                coluna(ui, LARGURA_LEITURA, |ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    ui.add_space(4.0);

                    if let Some(livro) = &self.livro_em_escolha {
                        ui.horizontal(|ui| {
                            if botao_icone(ui, Icone::Esq, p.texto).clicked() {
                                voltar = true;
                            }
                            ui.label(
                                RichText::new(&livro.name)
                                    .font(FontId::new(26.0, serif()))
                                    .strong(),
                            );
                        });
                        ui.label(
                            RichText::new(format!("{} capítulos", self.caps_escolha))
                                .color(p.suave),
                        );
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            for c in 1..=self.caps_escolha {
                                let ativo = livro.id == self.id_livro && c == self.capitulo;
                                if chip(ui, &c.to_string(), ativo, p, vec2(54.0, 48.0)).clicked() {
                                    ir = Some((livro.id, c));
                                }
                            }
                        });
                    } else {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.filtro_livro)
                                .hint_text("Filtrar livros…")
                                .desired_width(f32::INFINITY)
                                .margin(Margin::symmetric(14, 10)),
                        );
                        ui.add_space(6.0);

                        let filtro = self.filtro_livro.trim().to_lowercase();
                        let secoes: Vec<(&str, &[Livro])> = if self.lista_livros.len() == 66 {
                            let (at, nt) = self.lista_livros.split_at(39);
                            vec![("ANTIGO TESTAMENTO", at), ("NOVO TESTAMENTO", nt)]
                        } else {
                            vec![("LIVROS", &self.lista_livros[..])]
                        };

                        for (titulo, livros) in secoes {
                            let visiveis: Vec<&Livro> = livros
                                .iter()
                                .filter(|l| {
                                    filtro.is_empty()
                                        || l.name.to_lowercase().contains(&filtro)
                                        || l.abbrev.to_lowercase().contains(&filtro)
                                })
                                .collect();
                            if visiveis.is_empty() {
                                continue;
                            }
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(titulo)
                                    .size(12.0)
                                    .strong()
                                    .extra_letter_spacing(1.5)
                                    .color(p.suave),
                            );
                            ui.horizontal_wrapped(|ui| {
                                for l in visiveis {
                                    let ativo = l.id == self.id_livro;
                                    if chip(ui, &l.name, ativo, p, vec2(0.0, 42.0)).clicked() {
                                        escolha = Some(l.clone());
                                    }
                                }
                            });
                        }
                    }
                    ui.add_space(24.0);
                });
            });

        if voltar {
            self.livro_em_escolha = None;
        }
        if let Some(l) = escolha {
            self.caps_escolha = self.consultar_total_caps(l.id);
            self.livro_em_escolha = Some(l);
        }
        if let Some((id, c)) = ir {
            self.ir_para(id, c, None);
        }
    }

    // ───────────────────────── Busca ─────────────────────────

    fn ui_busca(&mut self, ui: &mut Ui, p: &Paleta) {
        if let Ok(novos) = self.rx_busca.try_recv() {
            self.resultados = novos;
            self.buscando = false;
            self.busca_feita = true;
        }
        if self.buscando {
            ui.ctx().request_repaint_after(Duration::from_millis(60));
        }

        let mut disparar = false;
        let mut destino: Option<(i32, i32, i32)> = None;
        let termo = self.termo_busca.replace('"', " ").trim().to_string();
        let tam = self.tamanho_fonte;

        coluna(ui, LARGURA_LEITURA, |ui| {
            ui.add_space(4.0);
            Frame::NONE
                .fill(p.superficie)
                .corner_radius(28.0)
                .stroke(Stroke::new(1.0, p.borda))
                .inner_margin(Margin::symmetric(16, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
                        desenhar_icone(ui.painter(), r.center(), Icone::Busca, p.suave, 18.0);

                        let edit = ui.add(
                            egui::TextEdit::singleline(&mut self.termo_busca)
                                .hint_text("Buscar palavra ou frase…")
                                //.frame(false)
                                .desired_width((ui.available_width() - 30.0).max(50.0)),
                        );
                        if self.focar_busca {
                            edit.request_focus();
                            self.focar_busca = false;
                        }
                        if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            disparar = true;
                        }
                        if self.buscando {
                            ui.add(egui::Spinner::new().size(18.0).color(p.acento));
                        }
                    });
                });
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 10.0;

                    if !self.busca_feita && !self.buscando {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("Busque por uma palavra ou frase exata")
                                    .font(FontId::new(18.0, serif()))
                                    .color(p.suave),
                            );
                            ui.label(RichText::new("ex.: amor, fé, “Jesus chorou”").color(p.suave));
                        });
                    } else if self.resultados.is_empty() && !self.buscando {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("Nenhum resultado encontrado").color(p.suave));
                        });
                    } else {
                        if !self.resultados.is_empty() {
                            let n = self.resultados.len();
                            let t = if n >= 50 {
                                "Mostrando os 50 resultados mais relevantes".to_string()
                            } else {
                                format!("{n} resultado(s)")
                            };
                            ui.label(RichText::new(t).size(13.0).color(p.suave));
                        }
                        for r in &self.resultados {
                            let resp = cartao(ui, p, |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}:{}",
                                        r.livro_nome, r.capitulo, r.numero
                                    ))
                                    .size(14.0)
                                    .strong()
                                    .color(p.acento),
                                );
                                ui.add(egui::Label::new(job_busca(&r.texto, &termo, p, tam)));
                            });
                            if resp.clicked() {
                                destino = Some((r.livro_id, r.capitulo, r.numero));
                            }
                        }
                    }
                    ui.add_space(24.0);
                });
        });

        if disparar {
            self.executar_busca_async();
        }
        if let Some((id, cap, v)) = destino {
            self.ir_para(id, cap, Some(v));
        }
    }

    // ───────────────────────── Salvos ─────────────────────────

    fn ui_salvos(&mut self, ui: &mut Ui, p: &Paleta) {
        let mut novo_filtro = None;
        let mut destino: Option<(i32, i32, i32)> = None;
        let tam = self.tamanho_fonte;

        coluna(ui, LARGURA_LEITURA, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (f, rot) in [
                    (FiltroSalvos::Favoritos, "Favoritos"),
                    (FiltroSalvos::Destaques, "Destaques"),
                ] {
                    if chip(ui, rot, self.filtro_salvos == f, p, vec2(110.0, 40.0)).clicked() {
                        novo_filtro = Some(f);
                    }
                }
            });
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 10.0;

                    if self.salvos.is_empty() {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            let (t, d) = match self.filtro_salvos {
                                FiltroSalvos::Favoritos => (
                                    "Nenhum favorito ainda",
                                    "Toque em um versículo e depois na estrela.",
                                ),
                                FiltroSalvos::Destaques => (
                                    "Nenhum destaque ainda",
                                    "Toque em um versículo e escolha uma cor.",
                                ),
                            };
                            ui.label(
                                RichText::new(t)
                                    .font(FontId::new(18.0, serif()))
                                    .color(p.suave),
                            );
                            ui.label(RichText::new(d).color(p.suave));
                        });
                    }

                    for s in &self.salvos {
                        let resp = cartao(ui, p, |ui| {
                            ui.horizontal(|ui| {
                                if let Some(hex) = &s.cor {
                                    let (r, _) =
                                        ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                    ui.painter().circle_filled(r.center(), 6.0, hex_cor(hex));
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}:{}",
                                        s.livro_nome, s.capitulo, s.numero
                                    ))
                                    .size(14.0)
                                    .strong()
                                    .color(p.acento),
                                );
                                if s.favorito {
                                    let (r, _) =
                                        ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                                    desenhar_icone(
                                        ui.painter(),
                                        r.center(),
                                        Icone::EstrelaCheia,
                                        Color32::from_rgb(222, 164, 30),
                                        14.0,
                                    );
                                }
                            });
                            ui.add(egui::Label::new(job_busca(&s.texto, "", p, tam)));
                        });
                        if resp.clicked() {
                            destino = Some((s.livro_id, s.capitulo, s.numero));
                        }
                    }
                    ui.add_space(24.0);
                });
        });

        if let Some(f) = novo_filtro {
            self.filtro_salvos = f;
            self.carregar_salvos();
        }
        if let Some((id, cap, v)) = destino {
            self.ir_para(id, cap, Some(v));
        }
    }

    // ───────────────────────── Ajustes (popover "Aa") ─────────────────────────

    fn ui_ajustes(&mut self, ctx: &egui::Context, p: &Paleta) {
        if !self.ajustes_abertos {
            return;
        }
        let mut novo_tema = None;
        let mut tam = self.tamanho_fonte;
        let mut salvar_fonte = false;
        let exemplo = Versiculo {
            numero: 1,
            texto: "No princípio, criou Deus os céus e a terra.".into(),
            cor: None,
            favorito: false,
        };
        let tema_atual = self.tema;

        let resp = egui::Window::new("ajustes")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::RIGHT_TOP, vec2(-12.0, SAFE_TOP as f32 + 64.0))
            .frame(Frame::window(&ctx.global_style()).inner_margin(Margin::same(18)))
            .show(ctx, |ui| {
                ui.set_width(280.0);
                ui.spacing_mut().item_spacing = vec2(8.0, 10.0);

                ui.label(
                    RichText::new("TEMA")
                        .size(12.0)
                        .strong()
                        .extra_letter_spacing(1.5)
                        .color(p.suave),
                );
                ui.horizontal(|ui| {
                    for t in Tema::TODOS {
                        if chip(ui, t.rotulo(), t == tema_atual, p, vec2(80.0, 40.0)).clicked() {
                            novo_tema = Some(t);
                        }
                    }
                });

                ui.add_space(6.0);
                ui.label(
                    RichText::new("TAMANHO DO TEXTO")
                        .size(12.0)
                        .strong()
                        .extra_letter_spacing(1.5)
                        .color(p.suave),
                );
                ui.horizontal(|ui| {
                    ui.label(RichText::new("A").size(14.0).color(p.suave));
                    ui.spacing_mut().slider_width = 170.0;
                    let r = ui.add(egui::Slider::new(&mut tam, 14.0..=34.0).show_value(false));
                    if r.drag_stopped() || (r.changed() && !r.dragged()) {
                        salvar_fonte = true;
                    }
                    ui.label(RichText::new("A").size(24.0).color(p.suave));
                });

                ui.add_space(6.0);
                Frame::NONE
                    .fill(p.fundo)
                    .corner_radius(12.0)
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(job_versiculo(&exemplo, p, tam)));
                    });
            });

        self.tamanho_fonte = tam;
        if salvar_fonte {
            self.salvar_config("tamanho_fonte", &tam.to_string());
        }
        if let Some(t) = novo_tema {
            self.tema = t;
            self.estilo_sujo = true;
            self.salvar_config("tema", t.chave());
        }

        // Fecha ao tocar fora do popover (o botão Aa faz o toggle sozinho)
        if let Some(r) = resp {
            let rect = r.response.rect;
            if ctx.input(|i| i.pointer.any_pressed()) {
                if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    if !rect.contains(pos) && !self.rect_aa.contains(pos) {
                        self.ajustes_abertos = false;
                    }
                }
            }
        }
    }
}

// ───────────────────────── App ─────────────────────────

impl eframe::App for BibliaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.estilo_sujo {
            tema::aplicar(ui.ctx(), self.tema);
            self.estilo_sujo = false;
        }
        let p = self.tema.paleta();

        self.tratar_voltar(ui.ctx());
        self.barra_topo(ui, &p);
        self.barra_inferior(ui, &p);

        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(p.fundo)
                    .inner_margin(Margin::symmetric(0, 0)),
            )
            .show_inside(ui, |ui| match self.tela {
                Tela::Leitura => self.ui_leitura(ui, &p),
                Tela::Livros => self.ui_livros(ui, &p),
                Tela::Busca => self.ui_busca(ui, &p),
                Tela::Salvos => self.ui_salvos(ui, &p),
            });

        self.ui_ajustes(ui.ctx(), &p);

        if self.aguardando_saida {
            egui::Area::new(egui::Id::new("toast_saida"))
                .order(egui::Order::Foreground)
                .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -96.0))
                .show(ui.ctx(), |ui| {
                    Frame::NONE
                        .fill(p.texto)
                        .corner_radius(22.0)
                        .inner_margin(Margin::symmetric(18, 10))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("Toque voltar novamente para sair").color(p.fundo),
                            );
                        });
                });
        }
    }
}

// ───────────────────────── Helpers de layout ─────────────────────────

/// Coluna centralizada com largura máxima (leitura confortável no desktop/web,
/// margens de 16px no celular). O conteúdo continua alinhado à esquerda.
fn coluna<R>(ui: &mut Ui, max: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let w = ui.available_width();
    let col = (w - 32.0).min(max).max(120.0);
    let lado = ((w - col) / 2.0).max(0.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(lado);
        ui.vertical(|ui| {
            ui.set_width(col);
            add(ui)
        })
        .inner
    })
    .inner
}

/// Cartão clicável (resultados de busca e salvos).
fn cartao(ui: &mut Ui, p: &Paleta, add: impl FnOnce(&mut Ui)) -> egui::Response {
    let r = Frame::NONE
        .fill(p.superficie)
        .corner_radius(16.0)
        .stroke(Stroke::new(1.0, p.borda))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 6.0;
            add(ui)
        });
    r.response
        .interact(Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Botão-título "Gênesis 1 ⌄" que abre o seletor de livros.
fn pilula(ui: &mut Ui, texto: &str, p: &Paleta) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(texto.to_owned(), FontId::new(18.0, serif()), p.texto);
    let tam = vec2(galley.size().x + 16.0 + 30.0, 40.0);
    let (rect, resp) = ui.allocate_exact_size(tam, Sense::click());

    let fundo = if resp.hovered() { p.borda } else { p.elevada };
    ui.painter().rect_filled(rect, 20.0, fundo);
    let pos = pos2(rect.left() + 16.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, p.texto);
    desenhar_icone(
        ui.painter(),
        pos2(rect.right() - 18.0, rect.center().y),
        Icone::Baixo,
        p.suave,
        12.0,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ───────────────────────── Texto rico ─────────────────────────

/// Versículo: número pequeno elevado na cor de acento + texto em serifa
/// com entrelinha generosa, marca-texto translúcido e estrela se favorito.
fn job_versiculo(v: &Versiculo, p: &Paleta, tam: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let linha = Some(tam * 1.65);

    job.append(
        &v.numero.to_string(),
        0.0,
        TextFormat {
            font_id: FontId::new((tam * 0.6).max(11.0), egui::FontFamily::Proportional),
            color: p.acento,
            valign: Align::TOP,
            line_height: linha,
            ..Default::default()
        },
    );
    job.append(
        &format!("\u{2009}{}", v.texto),
        4.0,
        TextFormat {
            font_id: FontId::new(tam, serif()),
            color: p.texto,
            background: v
                .cor
                .as_deref()
                .map(|h| cor_destaque(h, p))
                .unwrap_or(Color32::TRANSPARENT),
            line_height: linha,
            ..Default::default()
        },
    );
    if v.favorito {
        job.append(
            " ★",
            0.0,
            TextFormat {
                font_id: FontId::new(tam * 0.8, egui::FontFamily::Proportional),
                color: Color32::from_rgb(222, 164, 30),
                line_height: linha,
                ..Default::default()
            },
        );
    }
    job
}

/// Texto de resultado com o termo buscado destacado.
fn job_busca(texto: &str, termo: &str, p: &Paleta, tam: f32) -> LayoutJob {
    let normal = TextFormat {
        font_id: FontId::new((tam * 0.9).max(14.0), serif()),
        color: p.texto,
        line_height: Some(tam * 1.5),
        ..Default::default()
    };
    let mut marca = normal.clone();
    marca.background = alpha(p.acento, 70);

    let mut job = LayoutJob::default();
    let t = termo.to_lowercase();
    let baixo = texto.to_lowercase();

    // Só destaca quando o lowercase preserva os índices (seguro p/ fatiar UTF-8)
    if t.is_empty() || baixo.len() != texto.len() {
        job.append(texto, 0.0, normal);
        return job;
    }

    let mut i = 0;
    while let Some(pos) = baixo[i..].find(&t) {
        let ini = i + pos;
        let fim = ini + t.len();
        if !texto.is_char_boundary(ini) || !texto.is_char_boundary(fim) {
            break;
        }
        job.append(&texto[i..ini], 0.0, normal.clone());
        job.append(&texto[ini..fim], 0.0, marca.clone());
        i = fim;
    }
    job.append(&texto[i..], 0.0, normal);
    job
}
