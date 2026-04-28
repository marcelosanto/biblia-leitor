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
    // 1. Tenta adicionar a coluna (falha silenciosamente se já existir)
    let _ = conn.execute("ALTER TABLE verses ADD COLUMN texto_busca TEXT", []);

    // 2. Cria o índice para buscas instantâneas
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_verses_texto_busca ON verses(texto_busca)",
        [],
    )?;

    // 3. Verifica se precisamos popular a coluna (checa se o primeiro versículo está nulo)
    let precisa_popular: bool = conn.query_row(
        "SELECT COUNT(*) FROM verses WHERE texto_busca IS NULL LIMIT 1",
        [],
        |row| Ok(row.get::<_, i64>(0)? > 0),
    )?;

    if precisa_popular {
        println!("Otimizando banco de dados... isso ocorre apenas uma vez.");

        conn.execute("BEGIN TRANSACTION", [])?; // <--- ISSO MUDA TUDO
        // Buscamos todos os textos para normalizar no Rust
        let mut stmt = conn.prepare("SELECT id, text FROM verses")?;
        let rows = stmt.query_map(params![], |row| {
            // Especifique os tipos explicitamente no get para ajudar o compilador
            let id: i32 = row.get(0)?;
            let texto: String = row.get(1)?;
            Ok((id, texto))
        })?;

        for row in rows {
            if let Ok((id, texto)) = row {
                let texto_limpo = crate::normalizar(&texto);

                // Use params! aqui para evitar o erro de Sized com strings
                conn.execute(
                    "UPDATE verses SET texto_busca = ?1 WHERE id = ?2",
                    params![texto_limpo, id],
                )?;
            }
        }

        conn.execute("COMMIT", [])?;
        println!("Otimização concluída!");
    }

    Ok(())
}
