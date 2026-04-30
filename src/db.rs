use rusqlite::params;
use std::fs;
use std::path::PathBuf;

pub fn get_db_path() -> PathBuf {
    // Capturamos o resultado do bloco condicional em uma variável
    let path: PathBuf = {
        #[cfg(target_os = "android")]
        {
            // No Android, usamos o caminho interno do app
            let internal_path = PathBuf::from("/data/data/rust.biblia_egui/files");

            if !internal_path.exists() {
                let _ = std::fs::create_dir_all(&internal_path);
            }

            let db_file = internal_path.join("biblia.db");

            // Se o arquivo não existe na pasta de dados, extraímos do binário
            if !db_file.exists() {
                let db_bytes = include_bytes!("../assets/biblia.db");
                std::fs::write(&db_file, db_bytes).expect("Falha ao gravar o banco de dados");
            }

            db_file // Retorno do bloco Android (sem ponto e vírgula)
        }

        #[cfg(not(target_os = "android"))]
        {
            // No Desktop, aponta para a pasta local
            PathBuf::from("assets/biblia.db") // Retorno do bloco Desktop (sem ponto e vírgula)
        }
    };

    path // Retorno final da função
}

pub fn otimizar_banco_se_necessario(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let fts_existe: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='verses_fts'",
        [],
        |row| Ok(row.get::<_, i64>(0)? > 0),
    )?;

    if !fts_existe {
        println!("Criando motor de busca avançada (FTS5)... Isso leva poucos segundos.");

        conn.execute("BEGIN TRANSACTION", [])?;

        conn.execute(
            "CREATE VIRTUAL TABLE verses_fts USING fts5(
                book UNINDEXED,
                chapter UNINDEXED,
                verse UNINDEXED,
                text,
                tokenize='unicode61 remove_diacritics 1'
            )",
            [],
        )?;

        conn.execute(
            "INSERT INTO verses_fts (rowid, book, chapter, verse, text)
             SELECT id, book, chapter, verse, text FROM verses",
            [],
        )?;

        conn.execute("COMMIT", [])?;
        println!("Motor de busca FTS5 configurado com sucesso!");
    }

    Ok(())
}
