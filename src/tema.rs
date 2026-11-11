//! Design system do app: paletas (Claro / Sépia / Escuro), estilo global do egui,
//! ícones desenhados via Painter (iguais em desktop, web e Android) e widgets pequenos.

use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke,
    pos2, vec2,
};

// ───────────────────────────── Paletas ─────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tema {
    Claro,
    Sepia,
    Escuro,
}

impl Tema {
    pub const TODOS: [Tema; 3] = [Tema::Claro, Tema::Sepia, Tema::Escuro];

    pub fn chave(self) -> &'static str {
        match self {
            Tema::Claro => "claro",
            Tema::Sepia => "sepia",
            Tema::Escuro => "escuro",
        }
    }

    pub fn de_chave(s: &str) -> Tema {
        match s {
            "escuro" => Tema::Escuro,
            "sepia" => Tema::Sepia,
            _ => Tema::Claro,
        }
    }

    pub fn rotulo(self) -> &'static str {
        match self {
            Tema::Claro => "Claro",
            Tema::Sepia => "Sépia",
            Tema::Escuro => "Escuro",
        }
    }

    pub fn paleta(self) -> Paleta {
        match self {
            Tema::Claro => Paleta {
                fundo: Color32::from_rgb(250, 247, 242),
                superficie: Color32::from_rgb(255, 255, 255),
                elevada: Color32::from_rgb(240, 235, 227),
                texto: Color32::from_rgb(36, 32, 29),
                suave: Color32::from_rgb(126, 117, 107),
                acento: Color32::from_rgb(154, 70, 52),
                sobre_acento: Color32::WHITE,
                borda: Color32::from_rgb(226, 219, 208),
                escuro: false,
            },
            Tema::Sepia => Paleta {
                fundo: Color32::from_rgb(243, 234, 214),
                superficie: Color32::from_rgb(250, 243, 226),
                elevada: Color32::from_rgb(233, 222, 196),
                texto: Color32::from_rgb(62, 48, 36),
                suave: Color32::from_rgb(130, 112, 90),
                acento: Color32::from_rgb(146, 84, 34),
                sobre_acento: Color32::WHITE,
                borda: Color32::from_rgb(219, 205, 176),
                escuro: false,
            },
            Tema::Escuro => Paleta {
                fundo: Color32::from_rgb(17, 17, 19),
                superficie: Color32::from_rgb(27, 27, 31),
                elevada: Color32::from_rgb(38, 38, 43),
                texto: Color32::from_rgb(231, 226, 217),
                suave: Color32::from_rgb(146, 141, 133),
                acento: Color32::from_rgb(221, 177, 113),
                sobre_acento: Color32::from_rgb(30, 22, 10),
                borda: Color32::from_rgb(50, 50, 56),
                escuro: true,
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct Paleta {
    pub fundo: Color32,
    pub superficie: Color32,
    pub elevada: Color32,
    pub texto: Color32,
    pub suave: Color32,
    pub acento: Color32,
    pub sobre_acento: Color32,
    pub borda: Color32,
    pub escuro: bool,
}

pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// "#RRGGBB" -> Color32 (mesmo formato salvo no banco).
pub fn hex_cor(hex: &str) -> Color32 {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return Color32::from_rgb(255, 248, 59);
    }
    let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(255);
    Color32::from_rgb(c(0), c(2), c(4))
}

/// Cor do marca-texto: translúcida para não "gritar" e funcionar no escuro.
pub fn cor_destaque(hex: &str, p: &Paleta) -> Color32 {
    alpha(hex_cor(hex), if p.escuro { 90 } else { 135 })
}

pub fn sombra(p: &Paleta) -> egui::Shadow {
    egui::Shadow {
        offset: [0, 6],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(if p.escuro { 150 } else { 38 }),
    }
}

// ───────────────────────────── Fontes ─────────────────────────────

/// Família usada no texto bíblico e títulos. Sem a feature `fonte_serif`
/// cai para a fonte padrão (o app compila e roda do mesmo jeito).
pub fn serif() -> FontFamily {
    if cfg!(feature = "fonte_serif") {
        FontFamily::Name("serif".into())
    } else {
        FontFamily::Proportional
    }
}

#[cfg(feature = "fonte_serif")]
pub fn carregar_fontes(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "serif".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Literata.ttf"
        ))),
    );
    // Mantém as fontes padrão como fallback (símbolos, ★, etc.)
    let mut lista = vec!["serif".to_owned()];
    lista.extend(fonts.families[&FontFamily::Proportional].clone());
    fonts
        .families
        .insert(FontFamily::Name("serif".into()), lista);
    ctx.set_fonts(fonts);
}

#[cfg(not(feature = "fonte_serif"))]
pub fn carregar_fontes(_ctx: &egui::Context) {}

// ───────────────────────────── Estilo global ─────────────────────────────

pub fn aplicar(ctx: &egui::Context, tema: Tema) {
    let p = tema.paleta();
    let mut v = if p.escuro {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    v.panel_fill = p.fundo;
    v.window_fill = p.superficie;
    v.extreme_bg_color = p.superficie;
    v.faint_bg_color = p.elevada;
    v.hyperlink_color = p.acento;
    v.selection.bg_fill = alpha(p.acento, 70);
    v.selection.stroke = Stroke::new(1.0, p.acento);
    v.window_stroke = Stroke::new(1.0, p.borda);
    v.window_corner_radius = 20.0.into();
    v.menu_corner_radius = 14.0.into();
    v.window_shadow = sombra(&p);
    v.popup_shadow = sombra(&p);

    let w = &mut v.widgets;
    for s in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        s.corner_radius = 12.0.into();
        s.fg_stroke.color = p.texto;
    }
    w.noninteractive.bg_fill = p.superficie;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.borda);
    w.inactive.bg_fill = p.elevada;
    w.inactive.weak_bg_fill = p.elevada;
    w.inactive.bg_stroke = Stroke::NONE;
    w.hovered.bg_fill = p.borda;
    w.hovered.weak_bg_fill = p.borda;
    w.hovered.bg_stroke = Stroke::NONE;
    w.active.bg_fill = alpha(p.acento, 70);
    w.active.weak_bg_fill = alpha(p.acento, 70);
    w.active.bg_stroke = Stroke::NONE;

    let mut style = (*ctx.global_style()).clone();
    style.visuals = v;

    // Texto da interface com tamanho fixo; o tamanho de leitura é independente.
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(16.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(15.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(12.5, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::new(24.0, serif()));

    style.spacing.item_spacing = vec2(10.0, 10.0);
    style.spacing.button_padding = vec2(14.0, 10.0);
    style.spacing.interact_size = vec2(44.0, 44.0);
    style.spacing.scroll = egui::style::ScrollStyle::floating();
    style.interaction.selectable_labels = false; // não briga com o toque nos versículos

    ctx.set_global_style(style);
}

// ───────────────────────────── Ícones ─────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Icone {
    Ler,
    Livros,
    Busca,
    Salvos,
    Esq,
    Dir,
    Baixo,
    Aa,
    Copiar,
    Estrela,
    EstrelaCheia,
    Limpar,
}

pub fn desenhar_icone(pt: &Painter, c: Pos2, ic: Icone, cor: Color32, tam: f32) {
    let s = tam / 2.0;
    let st = Stroke::new((tam / 11.0).max(1.6), cor);
    let q = |x: f32, y: f32| pos2(c.x + x * s, c.y + y * s);
    let retangulo =
        |x0: f32, y0: f32, x1: f32, y1: f32| vec![q(x0, y0), q(x1, y0), q(x1, y1), q(x0, y1)];

    match ic {
        Icone::Ler => {
            pt.add(Shape::closed_line(
                vec![q(-1.0, -0.7), q(-0.06, -0.4), q(-0.06, 0.9), q(-1.0, 0.6)],
                st,
            ));
            pt.add(Shape::closed_line(
                vec![q(1.0, -0.7), q(0.06, -0.4), q(0.06, 0.9), q(1.0, 0.6)],
                st,
            ));
        }
        Icone::Livros => {
            for y in [-0.6, 0.0, 0.6] {
                pt.line_segment([q(-0.9, y), q(0.9, y)], st);
            }
        }
        Icone::Busca => {
            pt.circle_stroke(q(-0.15, -0.15), 0.7 * s, st);
            pt.line_segment([q(0.38, 0.38), q(0.95, 0.95)], st);
        }
        Icone::Salvos => {
            pt.add(Shape::closed_line(
                vec![
                    q(-0.6, -0.95),
                    q(0.6, -0.95),
                    q(0.6, 0.95),
                    q(0.0, 0.45),
                    q(-0.6, 0.95),
                ],
                st,
            ));
        }
        Icone::Esq => {
            pt.add(Shape::line(
                vec![q(0.35, -0.8), q(-0.35, 0.0), q(0.35, 0.8)],
                st,
            ));
        }
        Icone::Dir => {
            pt.add(Shape::line(
                vec![q(-0.35, -0.8), q(0.35, 0.0), q(-0.35, 0.8)],
                st,
            ));
        }
        Icone::Baixo => {
            pt.add(Shape::line(
                vec![q(-0.6, -0.3), q(0.0, 0.3), q(0.6, -0.3)],
                st,
            ));
        }
        Icone::Aa => {
            pt.text(
                c,
                Align2::CENTER_CENTER,
                "Aa",
                FontId::proportional(tam * 0.95),
                cor,
            );
        }
        Icone::Copiar => {
            pt.add(Shape::closed_line(retangulo(-0.85, -0.85, 0.3, 0.3), st));
            pt.add(Shape::closed_line(retangulo(-0.3, -0.3, 0.85, 0.85), st));
        }
        Icone::Estrela | Icone::EstrelaCheia => {
            let pts: Vec<Pos2> = (0..10)
                .map(|i| {
                    let r = if i % 2 == 0 { 1.0 } else { 0.42 };
                    let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                    q(r * a.cos(), r * a.sin())
                })
                .collect();
            if ic == Icone::EstrelaCheia {
                for i in 0..10 {
                    pt.add(Shape::convex_polygon(
                        vec![c, pts[i], pts[(i + 1) % 10]],
                        cor,
                        Stroke::NONE,
                    ));
                }
            }
            pt.add(Shape::closed_line(pts, st));
        }
        Icone::Limpar => {
            pt.circle_stroke(c, 0.8 * s, st);
            pt.line_segment([q(-0.57, 0.57), q(0.57, -0.57)], st);
        }
    }
}

// ───────────────────────────── Widgets ─────────────────────────────

/// Botão só-ícone com área de toque de 44px e hover circular.
pub fn botao_icone(ui: &mut egui::Ui, ic: Icone, cor: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    if resp.hovered() {
        ui.painter()
            .circle_filled(rect.center(), 20.0, alpha(cor, 24));
    }
    desenhar_icone(ui.painter(), rect.center(), ic, cor, 20.0);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Pílula arredondada (livros, capítulos, filtros, temas).
pub fn chip(ui: &mut egui::Ui, texto: &str, ativo: bool, p: &Paleta, min: egui::Vec2) -> Response {
    let cor_txt = if ativo { p.sobre_acento } else { p.texto };
    let galley = ui
        .painter()
        .layout_no_wrap(texto.to_owned(), FontId::proportional(15.0), cor_txt);
    let tam = vec2((galley.size().x + 28.0).max(min.x), min.y);
    let (rect, resp) = ui.allocate_exact_size(tam, Sense::click());

    let fundo = if ativo {
        p.acento
    } else if resp.hovered() {
        p.borda
    } else {
        p.elevada
    };
    ui.painter().rect_filled(rect, tam.y / 2.0, fundo);
    let pos = rect.center() - galley.size() / 2.0;
    ui.painter().galley(pos, galley, cor_txt);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Aba da barra inferior: ícone + rótulo + indicador.
pub fn aba(ui: &mut egui::Ui, ic: Icone, rotulo: &str, ativa: bool, p: &Paleta) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 54.0), Sense::click());
    let cor = if ativa { p.acento } else { p.suave };
    desenhar_icone(
        ui.painter(),
        pos2(rect.center().x, rect.top() + 17.0),
        ic,
        cor,
        22.0,
    );
    ui.painter().text(
        pos2(rect.center().x, rect.bottom() - 10.0),
        Align2::CENTER_CENTER,
        rotulo,
        FontId::proportional(12.0),
        cor,
    );
    if ativa {
        let ind = Rect::from_center_size(pos2(rect.center().x, rect.top() + 1.5), vec2(26.0, 3.0));
        ui.painter().rect_filled(ind, 2.0, cor);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}
