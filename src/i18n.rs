use std::collections::HashMap;
use std::env;
use std::fs;
use std::sync::OnceLock;

use crate::config;

/// Dicionário achatado (chaves em notação de ponto) carregado uma única vez.
static TRANSLATIONS: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Fallback embutido mínimo, usado apenas se o arquivo de idioma não existir.
fn builtin_fallback() -> HashMap<String, String> {
    HashMap::from([
        ("errors.generic".to_string(), "Erro: {error}".to_string()),
        (
            "lang.current".to_string(),
            "Idioma atual: {lang}".to_string(),
        ),
        (
            "lang.changed".to_string(),
            "Idioma alterado para '{lang}'.".to_string(),
        ),
        (
            "lang.not_found".to_string(),
            "Arquivo de idioma '{lang}.json' não encontrado ao lado do executável.".to_string(),
        ),
    ])
}

/// Achata recursivamente um JSON aninhado em chaves "modulo.chave".
fn flatten(value: &serde_json::Value, prefix: &str, out: &mut HashMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(v, &key, out);
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        _ => {}
    }
}

fn exe_dir() -> Option<std::path::PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

fn load_translations() -> HashMap<String, String> {
    let language = config::get_language();
    let path = match exe_dir() {
        Some(dir) => dir.join(format!("{language}.json")),
        None => return builtin_fallback(),
    };
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(json) => {
                let mut map = HashMap::new();
                flatten(&json, "", &mut map);
                map
            }
            Err(_) => builtin_fallback(),
        },
        Err(_) => builtin_fallback(),
    }
}

fn translations() -> &'static HashMap<String, String> {
    TRANSLATIONS.get_or_init(load_translations)
}

/// Retorna o texto da chave; se não existir, retorna "[{key}]" (sem panic).
pub fn t(key: &str) -> String {
    translations()
        .get(key)
        .cloned()
        .unwrap_or_else(|| format!("[{key}]"))
}

/// Como `t`, mas substitui placeholders `{nome}` pelos valores fornecidos.
pub fn t_args(key: &str, args: &[(&str, &str)]) -> String {
    let mut text = t(key);
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}
