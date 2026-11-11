# Bíblia Egui 🦀📖

Uma aplicação moderna da Bíblia Sagrada desenvolvida em **Rust**, focada em performance e portabilidade. Usa o framework **egui/eframe** para uma interface reativa e **SQLite** para o texto bíblico no desktop e no Android. A mesma base de código também roda no **navegador (WebAssembly)**.

> 🌐 **Demo online:** `https://marcelosanto.github.io/biblia-leitor/`

## ✨ Funcionalidades

- **Leitura confortável:** coluna centralizada, fonte serifada (opcional), entrelinha generosa, número do versículo elevado e tamanho do texto ajustável.
- **Três temas:** Claro, Sépia e Escuro. A escolha fica salva.
- **Marca-texto e favoritos:** toque em um versículo para pintar com 4 cores, copiar ou favoritar. Os destaques são translúcidos e funcionam bem no tema escuro.
- **Tela "Salvos":** lista de favoritos e destaques, com acesso direto ao versículo.
- **Busca rápida:** busca por palavra ou frase exata, com o termo destacado nos resultados. No desktop e no Android usa SQLite FTS5 e ignora acentos. Na web é feita em memória.
- **Seletor de livros e capítulos:** pelo título no topo, com filtro por nome e divisão entre Antigo e Novo Testamento.
- **Continua de onde parou:** o app reabre no último capítulo lido.
- **Navegação fluida:** troca de capítulo por botões ou setas do teclado, e passa automaticamente para o livro vizinho nas pontas.
- **Multiplataforma:** Desktop (Linux/Windows/macOS), Android (APK) e Web (WASM).
- **Funciona offline**, sem necessidade de internet (exceto o carregamento inicial da versão web).

## 🛠️ Tecnologias

- [Rust](https://www.rust-lang.org/) (linguagem principal)
- [egui / eframe](https://github.com/emilk/egui) (interface gráfica)
- [Rusqlite](https://github.com/rusqlite/rusqlite) (SQLite no desktop e no Android)
- [Trunk](https://trunkrs.dev/) (build web / WASM)
- [Cargo APK](https://github.com/rust-mobile/cargo-apk) (build para Android)

## 📁 Estrutura do Projeto

```text
├── assets/
│   ├── biblia.db            # Banco SQLite com o texto (desktop/Android)
│   ├── biblia.json          # Gerado a partir do .db, usado só na web
│   └── fonts/Literata.ttf   # (opcional) fonte serifada
├── src/
│   ├── main.rs              # Ponto de entrada: Desktop e Web
│   ├── lib.rs               # App, telas (Leitura, Livros, Busca, Salvos) e entrada Android
│   ├── tema.rs              # Design system: paletas, estilo global, ícones e widgets
│   ├── db.rs                # Caminho do banco e criação do índice FTS5
│   └── storage/
│       ├── mod.rs           # Tipos compartilhados e escolha do backend
│       ├── native.rs        # Backend SQLite (desktop/Android)
│       └── web.rs           # Backend web: JSON embutido + localStorage
├── tools/exportar_json.py   # Converte biblia.db em biblia.json
├── index.html               # Página host do build web (Trunk)
├── .github/workflows/pages.yml  # Deploy automático no GitHub Pages
├── Cargo.toml               # Dependências e targets
└── AndroidManifest.xml      # Manifesto da versão mobile
```

## 🧩 Arquitetura

A interface (`lib.rs`) nunca acessa SQLite ou `localStorage` diretamente. Ela conversa apenas com `Storage`, que tem duas implementações escolhidas em tempo de compilação:

| | Desktop / Android | Web |
|---|---|---|
| Texto bíblico | SQLite (`biblia.db`) | `biblia.json` embutido no `.wasm` |
| Busca | FTS5 em thread separada | varredura em memória (sem acento) |
| Marcações e configurações | tabelas SQLite | `localStorage` do navegador |

O design visual (cores, espaçamentos, ícones desenhados via `Painter`) fica centralizado em `tema.rs`, o que mantém a aparência idêntica em todas as plataformas, sem depender de emojis do sistema.

## ⚙️ Como Compilar

### Pré-requisitos

- Rust (versão estável)
- Android SDK & NDK (para a versão mobile)
- Python 3 (para gerar o JSON da versão web)
- Bibliotecas de sistema (Linux): `libwayland-dev`, `libx11-dev`, `libxkbcommon-dev`

### Desktop

```bash
cargo run --release
```

Com a fonte serifada (coloque `assets/fonts/Literata.ttf` antes):

```bash
cargo run --release --features fonte_serif
```

### Android

Certifique-se de que o celular/emulador está conectado via ADB:

```bash
cargo apk run --lib
```

### Web

```bash
python3 tools/exportar_json.py        # gera assets/biblia.json
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve                           # http://127.0.0.1:8080
```

Build de produção: `trunk build --release` (saída em `dist/`).

### Publicar a demo no GitHub Pages

1. Garanta que `assets/biblia.db` está commitado.
2. No repositório, vá em **Settings → Pages → Source: GitHub Actions**.
3. Faça push na branch `main`. O workflow gera o JSON, compila o WASM e publica automaticamente.

## ⌨️ Atalhos (Desktop e Web)

| Tecla | Ação |
|---|---|
| ← / → | Capítulo anterior / próximo |
| Esc | Fecha painéis e seleção; volta para a leitura |

## 🏗️ Desafios Técnicos Superados

- **Gestão de assets no Android:** extração do banco SQLite do APK para a pasta interna do sistema na primeira execução.
- **Compilação condicional:** uso de `#[cfg(target_os = "android")]` e `#[cfg(target_arch = "wasm32")]` para alternar entre backends e comportamentos de cada plataforma.
- **Uma base de código, três alvos:** a camada `Storage` isola o acesso a dados, permitindo rodar o mesmo app com SQLite nativo ou com dados em memória no navegador.
- **Targeting:** múltiplos ABIs (`aarch64`, `x86_64`) para dispositivos reais e emuladores.
- **Performance de renderização:** o texto bíblico é montado com `LayoutJob` (número, texto, marca-texto e favorito em um único bloco), e o total de capítulos é cacheado em vez de consultado a cada frame.

## ⚠️ Sobre o texto bíblico

Verifique a licença da tradução usada em `biblia.db`. Para demos públicas, prefira traduções de domínio público, já que a versão web distribui o texto completo junto com o app.

## 📄 Licença

Distribuído sob a licença MIT. Veja `LICENSE` para mais informações.
