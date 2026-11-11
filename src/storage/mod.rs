//! Camada de dados com duas implementações escolhidas em tempo de compilação:
//! - desktop/Android: SQLite (rusqlite) + FTS5
//! - web (wasm32): Bíblia em JSON embutido + marcações/config no localStorage
//!
//! A UI (lib.rs) só conhece `Storage`, nunca SQLite ou localStorage diretamente.

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

#[derive(PartialEq, Clone, Copy)]
pub enum FiltroSalvos {
    Favoritos,
    Destaques,
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::Storage;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::Storage;
