mod layout;
mod remote_memory;
mod shell_locator;
mod types;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use types::DesktopProfile;

const PROFILES_DIR: &str = "profiles";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Erro: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<String, String> {
    match args.first().map(String::as_str) {
        Some("save") => {
            let name = profile_name_arg(args)?;
            let profile = layout::save_layout(&name).map_err(|e| format!("{e}"))?;
            save_profile(&profile)?;
            Ok(format!(
                "Perfil '{}' salvo com {} ícones ({}x{}).",
                profile.profile_name,
                profile.icons.len(),
                profile.resolution.width,
                profile.resolution.height
            ))
        }
        Some("restore") => {
            let name = profile_name_arg(args)?;
            let profile = load_profile(&name)?;
            let moved = layout::restore_layout(&profile).map_err(|e| format!("{e}"))?;
            Ok(format!(
                "Perfil '{}' restaurado: {} de {} ícones reposicionados.",
                name,
                moved,
                profile.icons.len()
            ))
        }
        Some("list") => {
            let profiles = list_profiles()?;
            if profiles.is_empty() {
                Ok("Nenhum perfil salvo.".to_string())
            } else {
                Ok(format!("Perfis salvos:\n{}", profiles.join("\n")))
            }
        }
        _ => Err(format!(
            "Uso: desk0k <comando>\n  save <perfil>    Salva o layout atual\n  restore <perfil> Restaura um layout salvo\n  list             Lista perfis salvos"
        )),
    }
}

fn profile_name_arg(args: &[String]) -> Result<String, String> {
    let name = args
        .get(1)
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| "Informe o nome do perfil.".to_string())?;
    if name.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
        return Err("Nome de perfil contém caracteres inválidos.".to_string());
    }
    Ok(name.clone())
}

fn profile_path(name: &str) -> PathBuf {
    Path::new(PROFILES_DIR).join(format!("{name}.json"))
}

fn save_profile(profile: &DesktopProfile) -> Result<(), String> {
    fs::create_dir_all(PROFILES_DIR).map_err(|e| format!("{e}"))?;
    let json = serde_json::to_string_pretty(profile).map_err(|e| format!("{e}"))?;
    fs::write(profile_path(&profile.profile_name), json).map_err(|e| format!("{e}"))
}

fn load_profile(name: &str) -> Result<DesktopProfile, String> {
    let path = profile_path(name);
    let data = fs::read_to_string(&path)
        .map_err(|_| format!("Perfil '{name}' não encontrado em {}.", path.display()))?;
    serde_json::from_str(&data).map_err(|e| format!("Perfil inválido: {e}"))
}

fn list_profiles() -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let dir = match fs::read_dir(PROFILES_DIR) {
        Ok(dir) => dir,
        Err(_) => return Ok(names),
    };
    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}
