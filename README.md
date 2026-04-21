
# Bíblia Egui 🦀📱

Uma aplicação moderna da Bíblia Sagrada desenvolvida em **Rust**, focada em performance extrema e portabilidade. O projeto utiliza o framework **egui/eframe** para uma interface de utilizador (UI) reativa e **SQLite** para a gestão eficiente dos textos bíblicos.

## 🚀 Funcionalidades

- **Multiplataforma:** Binários nativos para Desktop (Linux/Windows) e Android (APK).
- **Interface Adaptável:** - Menu lateral (Side Panel) otimizado para Desktop.
  - Menu inferior (Bottom Navigation) desenhado para uso com o polegar no Android.
- **Base de Dados Local:** Utiliza SQLite para consultas instantâneas, sem necessidade de internet.
- **Arquitetura Reativa:** Interface fluida com 60 FPS garantidos pelo Immediate Mode GUI do Rust.

## 🛠️ Tecnologias Utilizadas

- [Rust](https://www.rust-lang.org/) (Linguagem principal)
- [egui / eframe](https://github.com/emilk/egui) (Interface gráfica)
- [Rusqlite](https://github.com/rusqlite/rusqlite) (Interface SQLite)
- [Cargo APK](https://github.com/rust-mobile/cargo-apk) (Build para Android)

## 📁 Estrutura do Projeto

```text
├── assets/             # Banco de dados biblia.db e ícones
├── src/
│   ├── main.rs         # Ponto de entrada Desktop
│   ├── lib.rs          # Lógica central e entrada Android
│   ├── db.rs           # Gestão de ficheiros e conexão SQLite
│   └── app.rs          # Definição da interface e navegação
├── Cargo.toml          # Configuração de dependências e targets
└── AndroidManifest.xml # Manifesto para a versão mobile
```


## ⚙️ Como Compilar

### Pré-requisitos

-   Rust (versão estável)
    
-   Android SDK & NDK (para a versão mobile)
    
-   Bibliotecas de sistema (Linux): `libwayland-dev`, `libx11-dev`, `libxkbcommon-dev`
    

### Desktop

Bash

```
cargo run --release

```

### Android

Certifique-se de que o seu telemóvel/emulador está ligado via ADB:

Bash

```
cargo apk run --lib

```

## 🏗️ Desafios Técnicos Superados

-   **Gestão de Assets no Android:** Implementação de lógica para extrair o banco de dados SQLite do APK para a pasta interna do sistema (`/data/user/0/...`) na primeira execução.
    
-   **Compilação Condicional:** Uso extensivo de `#[cfg(target_os = "android")]` para alternar entre layouts de menus e drivers de renderização de forma nativa.
    
-   **Targeting:** Configuração de múltiplos ABIs (`aarch64`, `x86_64`) para suporte a dispositivos reais e emuladores.
    

## 📄 Licença

Distribuído sob a licença MIT. Veja `LICENSE` para mais informações.
