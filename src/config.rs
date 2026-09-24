use std::env;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn default_language() -> String {
    "pt-br".to_string()
}

/// Configuração persistente do CLI, salva em `config.json` ao lado do exe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    #[serde(default = "default_language")]
    pub language: String,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
        }
    }
}

/// Caminho do `config.json`: mesma pasta do executável.
pub fn config_path() -> Result<PathBuf, String> {
    let exe = env::current_exe().map_err(|e| format!("{e}"))?;
    let dir = exe
        .parent()
        .ok_or_else(|| "Não foi possível determinar a pasta do executável.".to_string())?;
    Ok(dir.join("config.json"))
}

/// Carrega a configuração; cria o arquivo com os padrões se não existir.
/// Se o arquivo existir mas estiver inválido, retorna os padrões.
pub fn load_or_create_config() -> CliConfig {
    let path = match config_path() {
        Ok(p) => p,
        Err(_) => return CliConfig::default(),
    };
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => {
            let config = CliConfig::default();
            let _ = save_config(&config);
            config
        }
    }
}

pub fn save_config(config: &CliConfig) -> Result<(), String> {
    let path = config_path()?;
    let json = serde_json::to_string_pretty(config).map_err(|e| format!("{e}"))?;
    fs::write(&path, json).map_err(|e| format!("{e}"))
}

pub fn get_language() -> String {
    load_or_create_config().language
}

pub fn set_language(language: &str) -> Result<(), String> {
    let mut config = load_or_create_config();
    config.language = language.to_string();
    save_config(&config)
}
