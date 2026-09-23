mod layout;
mod process_watcher;
mod remote_memory;
mod shell_locator;
mod startup;
mod types;
mod wait_drive;

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
        Some("wait-drive") => match args.get(1).map(String::as_str) {
            Some("on") | Some("true") => {
                let nome = args
                    .get(2)
                    .filter(|n| !n.trim().is_empty())
                    .ok_or_else(|| {
                        "Informe o perfil a restaurar: wait-drive on <perfil> [--timeout N]"
                            .to_string()
                    })?;
                validate_profile_name(nome)?;
                let timeout = timeout_arg(args)?;
                if profile_path(nome).exists() {
                    wait_drive::enable(nome, timeout.map(|t| t.as_secs()))
                } else {
                    wait_drive::enable(nome, timeout.map(|t| t.as_secs())).map(|msg| {
                        format!("{msg}\nAviso: o perfil '{nome}' ainda não está salvo; salve antes com 'save {nome}'.")
                    })
                }
            }
            Some("off") | Some("false") => wait_drive::disable(),
            Some("status") | None => wait_drive::status(),
            Some("run") => wait_drive_run(),
            Some(_) => Err(
                "Uso: DesktopORZ wait-drive [on <perfil> [--timeout N] | off | status]".to_string(),
            ),
        },
        Some("help") | Some("--help") | Some("-h") | None => Ok(help_text().to_string()),
        Some(outro) => Err(format!(
            "Comando desconhecido: '{outro}'.\n\n{}",
            help_text()
        )),
    }
}

fn help_text() -> &'static str {
    "DesktopORZ - gerenciador de layouts de ícones da área de trabalho

USO:
    DesktopORZ <comando> [argumentos]

COMANDOS:

  save <perfil>
      Salva o layout atual da área de trabalho como um perfil.
      Exemplo: DesktopORZ save casa

  restore <perfil>
      Restaura as posições dos ícones de um perfil salvo.
      Exemplo: DesktopORZ restore casa

  list
      Lista todos os perfis salvos.

  startup on <perfil>
      Inicia o DesktopORZ junto com o Windows, restaurando o perfil no login.
      Exemplo: DesktopORZ startup on casa

  startup off
      Desativa a inicialização automática com o Windows.

  startup status
      Mostra se a inicialização automática está ativa e qual perfil será restaurado.

  wait-drive on <perfil> [--timeout N]    (também: true em vez de on)
      Ativa a espera pelo Google Drive a cada login do Windows.
      Assim que o GoogleDriveFS.exe iniciar (mais uma pausa de
      estabilização de 5 segundos), o perfil é restaurado automaticamente.

      --timeout N   (opcional) desiste após N segundos se o Drive não iniciar.
                    Sem a flag, aguarda indefinidamente.

      Exemplos:
        DesktopORZ wait-drive on casa
        DesktopORZ wait-drive on casa --timeout 120

  wait-drive off    (também: false)
      Desativa a espera pelo Google Drive no login.

  wait-drive status
      Mostra se a função está ativada, qual perfil ela restaura
      e se o timeout está ativado.

  help
      Mostra esta ajuda."
}

fn timeout_arg(args: &[String]) -> Result<Option<std::time::Duration>, String> {
    match args.iter().position(|a| a == "--timeout" || a == "-t") {
        Some(pos) => {
            let segundos: u64 = args
                .get(pos + 1)
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| "Informe um número válido após --timeout (segundos).".to_string())?;
            Ok(Some(std::time::Duration::from_secs(segundos)))
        }
        None => Ok(None),
    }
}

fn validate_profile_name(name: &str) -> Result<(), String> {
    if name.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
        return Err("Nome de perfil contém caracteres inválidos.".to_string());
    }
    Ok(())
}

fn profile_name_arg(args: &[String]) -> Result<String, String> {
    let name = args
        .get(1)
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| "Informe o nome do perfil.".to_string())?;
    validate_profile_name(name)?;
    Ok(name.clone())
}

/// Execução interna acionada no login do Windows (chave Run), quando o
/// aguardar-drive está ativado: espera o Google Drive e restaura o perfil.
fn wait_drive_run() -> Result<String, String> {
    let config = wait_drive::load_config();
    if !config.enabled {
        return Err("Aguardar Google Drive está desativado.".to_string());
    }
    let perfil = config.profile.unwrap();
    let timeout = config.timeout_secs.map(std::time::Duration::from_secs);
    println!(
        "Aguardando o processo '{}' iniciar{}...",
        process_watcher::GOOGLE_DRIVE_PROCESS,
        timeout
            .map(|t| format!(" (timeout: {}s)", t.as_secs()))
            .unwrap_or_else(|| " (sem timeout)".to_string())
    );
    let esperou = process_watcher::wait_for_process(
        process_watcher::GOOGLE_DRIVE_PROCESS,
        timeout,
        std::time::Duration::from_secs(5),
    )?;
    println!(
        "Google Drive detectado após {}s. Restaurando o perfil '{}'...",
        esperou.as_secs(),
        perfil
    );
    let perfil_dados = load_profile(&perfil)?;
    let movidos = layout::restore_layout(&perfil_dados).map_err(|e| format!("{e}"))?;
    Ok(format!(
        "Perfil '{}' restaurado após o Google Drive iniciar: {} de {} ícones reposicionados.",
        perfil,
        movidos,
        perfil_dados.icons.len()
    ))
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
