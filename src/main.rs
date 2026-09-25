// DesktopORZ — Save and restore Windows desktop icon layouts
// Copyright (C) 2026 ThainanViniciusKatchan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

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

use types::{DesktopProfile, Resolution};

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
            let current = shell_locator::current_resolution();
            let mut profile =
                layout::save_layout(&name, current).map_err(|e| format!("{e}"))?;
            if let Ok(existing) = load_profile(&name) {
                profile.resolutions = existing.resolutions;
            }
            save_profile(&profile)?;
            Ok(format!(
                "Perfil '{}' salvo com {} ícones ({}x{}).",
                profile.profile_name,
                profile.icons.len(),
                profile.resolution.width,
                profile.resolution.height
            ))
        }
        Some("save-res") => {
            let name = profile_name_arg(args)?;
            let resolution = match args.get(2) {
                Some(raw) => parse_resolution(raw)?,
                None => shell_locator::current_resolution(),
            };
            let current = layout::save_layout(&name, resolution).map_err(|e| format!("{e}"))?;
            let key = format!("{}x{}", resolution.width, resolution.height);
            let count = current.icons.len();
            let mut profile = match load_profile(&name) {
                Ok(existing) => existing,
                Err(_) => current.clone(),
            };
            profile.resolutions.insert(key.clone(), current.icons);
            save_profile(&profile)?;
            Ok(format!(
                "Perfil '{}' salvo com {} ícones para a resolução {} (dentro de {}.json).",
                name,
                count,
                key,
                name
            ))
        }
        Some("profile-update") => {
            let name = profile_name_arg(args)?;
            let mut profile = load_profile(&name)
                .map_err(|_| format!("Perfil '{name}' não existe. Use 'save {name}' para criá-lo."))?;
            let current = shell_locator::current_resolution();
            let captured = layout::save_layout(&name, current).map_err(|e| format!("{e}"))?;
            profile.resolution = captured.resolution;
            profile.timestamp_utc = captured.timestamp_utc;
            profile.icons = captured.icons.clone();
            let key = format!("{}x{}", current.width, current.height);
            if profile.resolutions.contains_key(&key) {
                profile.resolutions.insert(key.clone(), captured.icons);
            }
            let count = profile.icons.len();
            save_profile(&profile)?;
            Ok(format!(
                "Perfil '{}' atualizado com {} ícones ({}x{}); layouts por resolução preservados.",
                name, count, current.width, current.height
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

  save-res <perfil> [LARGURAxALTURA]
      Salva o layout atual para uma resolução, dentro do mesmo perfil
      (campo 'resolutions' do arquivo do perfil).
      Sem a resolução informada, usa a resolução atual da área de trabalho.
      Exemplos:
        DesktopORZ save-res casa            (usa a resolução atual)
        DesktopORZ save-res casa 1920x1080  (marca o perfil para 1920x1080)

  profile-update <perfil>
      Atualiza um perfil já existente com o layout atual da área de
      trabalho, preservando os layouts por resolução salvos (save-res).
      Se houver um layout salvo para a resolução atual, ele também é
      atualizado.
      Exemplo: DesktopORZ profile-update casa

  restore <perfil>
      Restaura as posições dos ícones de um perfil salvo.
      Se o perfil tiver um layout salvo para a resolução atual
      (via save-res), ele é usado automaticamente; caso contrário,
      usa o layout base do perfil.
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
        process_watcher::GOOGLE_DRIVE_DESCRIPTION,
        timeout
            .map(|t| format!(" (timeout: {}s)", t.as_secs()))
            .unwrap_or_else(|| " (sem timeout)".to_string())
    );
    let esperou = process_watcher::wait_for_process(
        process_watcher::GOOGLE_DRIVE_DESCRIPTION,
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

fn parse_resolution(raw: &str) -> Result<Resolution, String> {
    let (w, h) = raw.split_once(['x', 'X']).ok_or_else(|| {
        format!("Resolução inválida: '{raw}'. Use o formato LARGURAxALTURA, ex.: 1920x1080")
    })?;
    let width: u32 = w.parse().map_err(|_| format!("Largura inválida em '{raw}'"))?;
    let height: u32 = h.parse().map_err(|_| format!("Altura inválida em '{raw}'"))?;
    if width == 0 || height == 0 {
        return Err("Resolução inválida: largura e altura devem ser maiores que zero".into());
    }
    Ok(Resolution { width, height })
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
