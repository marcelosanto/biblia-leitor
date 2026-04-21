use std::fs;
use std::path::PathBuf;

pub fn get_db_path() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        // O log indica que seu app é "rust.biblia_egui"
        let internal_path = PathBuf::from("/data/user/0/rust.biblia_egui/files");

        // 1. GARANTE que a pasta existe antes de tentar criar o arquivo
        if !internal_path.exists() {
            // create_dir_all cria todas as pastas do caminho se não existirem
            fs::create_dir_all(&internal_path)
                .expect("Não foi possível criar o diretório de dados");
        }

        let db_file = internal_path.join("biblia.db");

        // 2. Só copia se o arquivo não estiver lá
        if !db_file.exists() {
            let db_bytes = include_bytes!("../assets/biblia.db");
            // fs::write agora funcionará porque a pasta pai já existe
            fs::write(&db_file, db_bytes).expect("Falha ao gravar o banco de dados no Android");
        }
        db_file
    }

    #[cfg(not(target_os = "android"))]
    {
        PathBuf::from("assets/biblia.db")
    }
}
