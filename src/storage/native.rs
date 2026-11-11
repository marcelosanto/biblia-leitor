use std::sync::mpsc::Sender;

use rusqlite::Connection;

use super::{FiltroSalvos, Livro, ResultadoBusca, Salvo, Versiculo};

pub struct Storage;

fn abrir() -> Option<Connection> {
    Connection::open(crate::db::get_db_path()).ok()
}

impl Storage {
    /// Cria tabelas auxiliares e o índice FTS5 se necessário.
    pub fn new() -> Self {
        if let Some(conn) = abrir() {
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
        Storage
    }

    pub fn livros(&self) -> Vec<Livro> {
        let Some(conn) = abrir() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare("SELECT id, name, abbrev FROM books ORDER BY id") else {
            return Vec::new();
        };
        stmt.query_map([], |row| {
            Ok(Livro {
                id: row.get(0)?,
                name: row.get(1)?,
                abbrev: row.get(2)?,
            })
        })
        .map(|it| it.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    pub fn total_caps(&self, livro: i32) -> i32 {
        abrir()
            .and_then(|conn| {
                conn.query_row(
                    "SELECT COALESCE(MAX(chapter), 1) FROM verses WHERE book = ?1",
                    [livro],
                    |row| row.get::<_, i32>(0),
                )
                .ok()
            })
            .unwrap_or(1)
    }

    pub fn versiculos(&self, livro: i32, cap: i32) -> Vec<Versiculo> {
        let Some(conn) = abrir() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT v.verse, v.text, m.cor, m.favorito
             FROM verses v
             LEFT JOIN marcacoes m ON v.book = m.book AND v.chapter = m.chapter AND v.verse = m.verse
             WHERE v.book = ?1 AND v.chapter = ?2
             ORDER BY v.verse ASC",
        ) else {
            return Vec::new();
        };
        stmt.query_map([livro, cap], |row| {
            let fav: Option<i32> = row.get(3)?;
            Ok(Versiculo {
                numero: row.get(0)?,
                texto: row.get(1)?,
                cor: row.get::<_, Option<String>>(2)?,
                favorito: fav.unwrap_or(0) == 1,
            })
        })
        .map(|it| it.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    pub fn salvos(&self, filtro: FiltroSalvos) -> Vec<Salvo> {
        let cond = match filtro {
            FiltroSalvos::Favoritos => "m.favorito = 1",
            FiltroSalvos::Destaques => "m.cor IS NOT NULL",
        };
        let sql = format!(
            "SELECT m.book, b.name, m.chapter, m.verse, v.text, m.cor, m.favorito
             FROM marcacoes m
             JOIN verses v ON v.book = m.book AND v.chapter = m.chapter AND v.verse = m.verse
             JOIN books b ON b.id = m.book
             WHERE {cond}
             ORDER BY m.book, m.chapter, m.verse LIMIT 500"
        );
        let Some(conn) = abrir() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(&sql) else {
            return Vec::new();
        };
        stmt.query_map([], |row| {
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
        })
        .map(|it| it.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    /// Busca em thread separada (FTS5) e envia o resultado pelo canal.
    pub fn buscar_async(&self, termo: String, tx: Sender<Vec<ResultadoBusca>>) {
        std::thread::spawn(move || {
            let termo = termo.replace('"', " ").trim().to_string();
            let mut resultados = Vec::new();
            if !termo.is_empty() {
                if let Some(conn) = abrir() {
                    if let Ok(mut stmt) = conn.prepare(
                        "SELECT b.name, f.book, f.chapter, f.verse, f.text
                         FROM verses_fts f
                         JOIN books b ON f.book = b.id
                         WHERE f.text MATCH ?1
                         ORDER BY rank
                         LIMIT 50",
                    ) {
                        let frase = format!("\"{}\"", termo);
                        if let Ok(iter) = stmt.query_map([frase], |row| {
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
            }
            let _ = tx.send(resultados);
        });
    }

    pub fn salvar_marcacao(
        &self,
        livro: i32,
        cap: i32,
        verso: i32,
        cor: Option<&str>,
        fav: Option<bool>,
    ) {
        let Some(conn) = abrir() else { return };
        conn.execute(
            "INSERT OR IGNORE INTO marcacoes (book, chapter, verse, favorito) VALUES (?1, ?2, ?3, 0)",
            rusqlite::params![livro, cap, verso],
        )
        .ok();
        if let Some(c) = cor {
            conn.execute(
                "UPDATE marcacoes SET cor = ?1 WHERE book = ?2 AND chapter = ?3 AND verse = ?4",
                rusqlite::params![c, livro, cap, verso],
            )
            .ok();
        }
        if let Some(f) = fav {
            conn.execute(
                "UPDATE marcacoes SET favorito = ?1 WHERE book = ?2 AND chapter = ?3 AND verse = ?4",
                rusqlite::params![f as i32, livro, cap, verso],
            )
            .ok();
        }
    }

    /// Remove só a cor; se não sobrar nada (nem favorito), apaga a linha.
    pub fn limpar_cor(&self, livro: i32, cap: i32, verso: i32) {
        let Some(conn) = abrir() else { return };
        let p = rusqlite::params![livro, cap, verso];
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

    pub fn salvar_config(&self, chave: &str, valor: &str) {
        if let Some(conn) = abrir() {
            conn.execute(
                "INSERT OR REPLACE INTO config (chave, valor) VALUES (?1, ?2)",
                [chave, valor],
            )
            .ok();
        }
    }

    pub fn ler_config(&self, chave: &str, padrao: &str) -> String {
        abrir()
            .and_then(|conn| {
                conn.query_row("SELECT valor FROM config WHERE chave = ?1", [chave], |r| {
                    r.get::<_, String>(0)
                })
                .ok()
            })
            .unwrap_or_else(|| padrao.to_string())
    }
}
