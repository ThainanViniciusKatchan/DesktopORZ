mod layout;
mod remote_memory;
mod shell_locator;
mod startup;
mod types;

use std::env;
use std::fs;
use std::path::PathBuf;
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
        Some("startup") => match args.get(1).map(String::as_str) {
            Some("on") => {
                let name = args
                    .get(2)
                    .filter(|n| !n.trim().is_empty())
                    .ok_or_else(|| "Informe o perfil a restaurar no login: startup on <perfil>".to_string())?;
                if profile_path(name).exists() {
                    startup::enable(name)
                } else {
                    startup::enable(name).map(|msg| {
                        format!("{msg}\nAviso: o perfil '{name}' ainda não está salvo; salve antes com 'save {name}'.")
                    })
                }
            }
            Some("off") => startup::disable(),
            Some("status") | None => startup::status(),
            Some(_) => Err("Uso: desk0k startup [on <perfil> | off | status]".to_string()),
        },
        _ => Err(format!(
            "Uso: desk0k <comando>\n  save <perfil>       Salva o layout atual\n  restore <perfil>    Restaura um layout salvo\n  list                Lista perfis salvos\n  startup on <perfil> Inicia com o Windows restaurando o perfil\n  startup off         Desativa a inicialização automática\n  startup status      Mostra se a inicialização automática está ativa"
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

fn profiles_base_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(PROFILES_DIR)
}

fn profile_path(name: &str) -> PathBuf {
    profiles_base_dir().join(format!("{name}.json"))
}

fn save_profile(profile: &DesktopProfile) -> Result<(), String> {
    fs::create_dir_all(profiles_base_dir()).map_err(|e| format!("{e}"))?;
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
    let dir = match fs::read_dir(profiles_base_dir()) {
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
