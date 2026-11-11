//! Backend web: Bíblia em JSON embutido no .wasm, busca em memória
//! (sem acento / sem diferenciar maiúsculas) e marcações no localStorage.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::mpsc::Sender;

use serde::Deserialize;

use super::{FiltroSalvos, Livro, ResultadoBusca, Salvo, Versiculo};

// Gerado por tools/exportar_json.py
const JSON: &str = include_str!("../../assets/biblia.json");
const CHAVE_MARCAS: &str = "biblia:marcas";

#[derive(Deserialize)]
struct Dados {
    livros: Vec<(i32, String, String)>,
    versos: Vec<(i32, i32, i32, String)>, // (livro, capítulo, versículo, texto) ordenados
}

type Marca = (Option<String>, bool);
type MarcaSerial = (i32, i32, i32, Option<String>, bool);

pub struct Storage {
    dados: Dados,
    normalizados: Vec<String>,
    marcas: RefCell<BTreeMap<(i32, i32, i32), Marca>>,
}

fn local() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn normaliza(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            o => o,
        })
        .collect()
}

impl Storage {
    pub fn new() -> Self {
        let dados: Dados = serde_json::from_str(JSON).expect("assets/biblia.json inválido");
        let normalizados = dados.versos.iter().map(|v| normaliza(&v.3)).collect();

        let marcas = local()
            .and_then(|ls| ls.get_item(CHAVE_MARCAS).ok().flatten())
            .and_then(|txt| serde_json::from_str::<Vec<MarcaSerial>>(&txt).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|(b, c, v, cor, f)| ((b, c, v), (cor, f)))
            .collect();

        Storage {
            dados,
            normalizados,
            marcas: RefCell::new(marcas),
        }
    }

    fn persistir(&self) {
        let lista: Vec<MarcaSerial> = self
            .marcas
            .borrow()
            .iter()
            .map(|(&(b, c, v), (cor, f))| (b, c, v, cor.clone(), *f))
            .collect();
        if let (Some(ls), Ok(txt)) = (local(), serde_json::to_string(&lista)) {
            let _ = ls.set_item(CHAVE_MARCAS, &txt);
        }
    }

    fn faixa(&self, livro: i32, cap: i32) -> Range<usize> {
        let v = &self.dados.versos;
        let ini = v.partition_point(|x| (x.0, x.1) < (livro, cap));
        let fim = v.partition_point(|x| (x.0, x.1) <= (livro, cap));
        ini..fim
    }

    fn nome_livro(&self, id: i32) -> String {
        self.dados
            .livros
            .iter()
            .find(|l| l.0 == id)
            .map(|l| l.1.clone())
            .unwrap_or_default()
    }

    pub fn livros(&self) -> Vec<Livro> {
        self.dados
            .livros
            .iter()
            .map(|l| Livro {
                id: l.0,
                name: l.1.clone(),
                abbrev: l.2.clone(),
            })
            .collect()
    }

    pub fn total_caps(&self, livro: i32) -> i32 {
        let v = &self.dados.versos;
        let fim = v.partition_point(|x| x.0 <= livro);
        if fim > 0 && v[fim - 1].0 == livro {
            v[fim - 1].1
        } else {
            1
        }
    }

    pub fn versiculos(&self, livro: i32, cap: i32) -> Vec<Versiculo> {
        let marcas = self.marcas.borrow();
        self.dados.versos[self.faixa(livro, cap)]
            .iter()
            .map(|v| {
                let (cor, favorito) = marcas
                    .get(&(v.0, v.1, v.2))
                    .cloned()
                    .unwrap_or((None, false));
                Versiculo {
                    numero: v.2,
                    texto: v.3.clone(),
                    cor,
                    favorito,
                }
            })
            .collect()
    }

    pub fn salvos(&self, filtro: FiltroSalvos) -> Vec<Salvo> {
        let marcas = self.marcas.borrow();
        marcas
            .iter()
            .filter(|(_, (cor, fav))| match filtro {
                FiltroSalvos::Favoritos => *fav,
                FiltroSalvos::Destaques => cor.is_some(),
            })
            .filter_map(|(&(b, c, v), (cor, fav))| {
                let i = self
                    .dados
                    .versos
                    .binary_search_by_key(&(b, c, v), |x| (x.0, x.1, x.2))
                    .ok()?;
                Some(Salvo {
                    livro_id: b,
                    livro_nome: self.nome_livro(b),
                    capitulo: c,
                    numero: v,
                    texto: self.dados.versos[i].3.clone(),
                    cor: cor.clone(),
                    favorito: *fav,
                })
            })
            .take(500)
            .collect()
    }

    /// Na web não há threads: busca em memória (~31 mil versículos, instantâneo)
    /// e responde pelo mesmo canal usado no desktop.
    pub fn buscar_async(&self, termo: String, tx: Sender<Vec<ResultadoBusca>>) {
        let t = normaliza(&termo.replace('"', " "));
        let t = t.trim();
        let mut res = Vec::new();
        if !t.is_empty() {
            for (i, n) in self.normalizados.iter().enumerate() {
                if n.contains(t) {
                    let v = &self.dados.versos[i];
                    res.push(ResultadoBusca {
                        livro_nome: self.nome_livro(v.0),
                        livro_id: v.0,
                        capitulo: v.1,
                        numero: v.2,
                        texto: v.3.clone(),
                    });
                    if res.len() >= 50 {
                        break;
                    }
                }
            }
        }
        let _ = tx.send(res);
    }

    pub fn salvar_marcacao(
        &self,
        livro: i32,
        cap: i32,
        verso: i32,
        cor: Option<&str>,
        fav: Option<bool>,
    ) {
        {
            let mut m = self.marcas.borrow_mut();
            let e = m.entry((livro, cap, verso)).or_insert((None, false));
            if let Some(c) = cor {
                e.0 = Some(c.to_string());
            }
            if let Some(f) = fav {
                e.1 = f;
            }
        }
        self.persistir();
    }

    pub fn limpar_cor(&self, livro: i32, cap: i32, verso: i32) {
        {
            let mut m = self.marcas.borrow_mut();
            if let Some(e) = m.get_mut(&(livro, cap, verso)) {
                e.0 = None;
                if !e.1 {
                    m.remove(&(livro, cap, verso));
                }
            }
        }
        self.persistir();
    }

    pub fn salvar_config(&self, chave: &str, valor: &str) {
        if let Some(ls) = local() {
            let _ = ls.set_item(&format!("biblia:cfg:{chave}"), valor);
        }
    }

    pub fn ler_config(&self, chave: &str, padrao: &str) -> String {
        local()
            .and_then(|ls| ls.get_item(&format!("biblia:cfg:{chave}")).ok().flatten())
            .unwrap_or_else(|| padrao.to_string())
    }
}
