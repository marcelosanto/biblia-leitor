use std::fs;
use std::path::PathBuf;

pub fn get_db_path() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        let internal_path = PathBuf::from("/data/user/0/rust.biblia_egui/files");

        if !internal_path.exists() {
            fs::create_dir_all(&internal_path)
                .expect("Não foi possível criar o diretório de dados");
        }

        let db_file = internal_path.join("biblia.db");

        if !db_file.exists() {
            let db_bytes = include_bytes!("../assets/biblia.db");
            fs::write(&db_file, db_bytes).expect("Falha ao gravar o banco de dados no Android");
        }
        db_file
    }

    #[cfg(not(target_os = "android"))]
    {
        PathBuf::from("assets/biblia.db")
    }
}
