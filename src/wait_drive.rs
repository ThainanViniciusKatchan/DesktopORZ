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

use std::env;
use std::path::PathBuf;

use windows::core::w;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
    REG_CREATE_KEY_DISPOSITION,
    RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_DWORD,
    REG_OPTION_NON_VOLATILE, REG_SAM_FLAGS, REG_SZ,
};

const APP_KEY: windows::core::PCWSTR = w!("Software\\DesktopORZ");
const RUN_KEY: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE_PROFILE: windows::core::PCWSTR = w!("WaitDriveProfile");
const VALUE_TIMEOUT: windows::core::PCWSTR = w!("WaitDriveTimeout");
const RUN_VALUE_NAME: windows::core::PCWSTR = w!("DesktopORZ-WaitDrive");

#[derive(Default)]
pub struct WaitDriveConfig {
    pub enabled: bool,
    pub profile: Option<String>,
    /// Timeout em segundos; None = sem timeout (aguarda indefinidamente).
    pub timeout_secs: Option<u64>,
}

fn exe_path() -> Result<PathBuf, String> {
    let p = env::current_exe()
        .map(|p| p.canonicalize().unwrap_or(p))
        .map_err(|e| format!("{e}"))?;
    // Remove o prefixo estendido `\\?\` que o gerenciador da chave Run não entende.
    if let Some(s) = p.to_str() {
        if let Some(rest) = s.strip_prefix("\\\\?\\") {
            return Ok(PathBuf::from(rest));
        }
    }
    Ok(p)
}

fn open_key(
    root: HKEY,
    subkey: windows::core::PCWSTR,
    access: REG_SAM_FLAGS,
) -> Result<HKEY, String> {
    unsafe {
        let mut key = HKEY::default();
        let status = RegOpenKeyExW(root, subkey, 0, access, &mut key);
        if status.is_err() {
            return Err(format!("Não foi possível abrir a chave do registro: {status:?}"));
        }
        Ok(key)
    }
}

fn open_app_key(access: REG_SAM_FLAGS) -> Result<HKEY, String> {
    if access == KEY_SET_VALUE {
        unsafe {
            let mut key = HKEY::default();
            let mut disposition = REG_CREATE_KEY_DISPOSITION(0);
            let status = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                APP_KEY,
                0,
                windows::core::PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                Some(&mut disposition as *mut _),
            );
            if status.is_err() {
                return Err(format!(
                    "Não foi possível criar a chave do DesktopORZ no registro: {status:?}"
                ));
            }
            return Ok(key);
        }
    }
    open_key(HKEY_CURRENT_USER, APP_KEY, access)
}

fn set_sz(key: HKEY, name: windows::core::PCWSTR, value: &str) -> Result<(), String> {
    let mut data: Vec<u16> = value.encode_utf16().collect();
    data.push(0);
    unsafe {
        let status = RegSetValueExW(
            key,
            name,
            0,
            REG_SZ,
            Some(std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * 2,
            )),
        );
        if status.is_err() {
            return Err(format!("Falha ao gravar no registro: {status:?}"));
        }
    }
    Ok(())
}

fn set_dword(key: HKEY, name: windows::core::PCWSTR, value: u32) -> Result<(), String> {
    unsafe {
        let status = RegSetValueExW(
            key,
            name,
            0,
            REG_DWORD,
            Some(std::slice::from_raw_parts(
                &value as *const u32 as *const u8,
                std::mem::size_of::<u32>(),
            )),
        );
        if status.is_err() {
            return Err(format!("Falha ao gravar no registro: {status:?}"));
        }
    }
    Ok(())
}

fn delete_value(key: HKEY, name: windows::core::PCWSTR) {
    unsafe {
        let _ = RegDeleteValueW(key, name);
    }
}

fn query_sz(key: HKEY, name: windows::core::PCWSTR) -> Option<String> {
    unsafe {
        let mut size: u32 = 0;
        if RegQueryValueExW(key, name, None, None, None, Some(&mut size)).is_err() {
            return None;
        }
        if size < 2 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        if RegQueryValueExW(key, name, None, None, Some(buf.as_mut_ptr()), Some(&mut size))
            .is_err()
        {
            return None;
        }
        let wide = std::slice::from_raw_parts(buf.as_ptr() as *const u16, (size as usize) / 2);
        let text = String::from_utf16_lossy(wide).trim_end_matches('\0').to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

fn query_dword(key: HKEY, name: windows::core::PCWSTR) -> Option<u32> {
    unsafe {
        let mut value: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let status = RegQueryValueExW(
            key,
            name,
            None,
            None,
            Some(&mut value as *mut u32 as *mut u8),
            Some(&mut size),
        );
        if status.is_err() {
            None
        } else {
            Some(value)
        }
    }
}

fn register_run_entry() -> Result<(), String> {
    let exe = exe_path()?;
    let cmd = format!("\"{}\" wait-drive run", exe.display());
    unsafe {
        let key = open_key(HKEY_CURRENT_USER, RUN_KEY, KEY_SET_VALUE)?;
        let result = set_sz(key, RUN_VALUE_NAME, &cmd);
        let _ = RegCloseKey(key);
        result
    }
}

fn remove_run_entry() {
    if let Ok(key) = open_key(HKEY_CURRENT_USER, RUN_KEY, KEY_SET_VALUE) {
        delete_value(key, RUN_VALUE_NAME);
        unsafe {
            let _ = RegCloseKey(key);
        }
    }
}

pub fn load_config() -> WaitDriveConfig {
    let mut config = WaitDriveConfig::default();
    if let Ok(key) = open_app_key(KEY_QUERY_VALUE) {
        config.profile = query_sz(key, VALUE_PROFILE);
        config.timeout_secs = query_dword(key, VALUE_TIMEOUT).and_then(|v| {
            if v == 0 {
                None
            } else {
                Some(v as u64)
            }
        });
        config.enabled = config.profile.is_some();
        unsafe {
            let _ = RegCloseKey(key);
        }
    }
    config
}

pub fn enable(profile: &str, timeout_secs: Option<u64>) -> Result<String, String> {
    let key = open_app_key(KEY_SET_VALUE)?;
    let result = set_sz(key, VALUE_PROFILE, profile).and_then(|_| {
        set_dword(key, VALUE_TIMEOUT, timeout_secs.unwrap_or(0) as u32)
    });
    unsafe {
        let _ = RegCloseKey(key);
    }
    result?;
    register_run_entry()?;
    Ok(format!(
        "Aguardar Google Drive: ATIVADO.\nPerfil: {profile}\nTimeout: {}\nO DesktopORZ vai aguardar o Google Drive iniciar a cada login do Windows.",
        timeout_secs
            .map(|t| format!("{t}s"))
            .unwrap_or_else(|| "desativado (sem limite)".to_string())
    ))
}

pub fn disable() -> Result<String, String> {
    let key = open_app_key(KEY_SET_VALUE)?;
    delete_value(key, VALUE_PROFILE);
    delete_value(key, VALUE_TIMEOUT);
    unsafe {
        let _ = RegCloseKey(key);
    }
    remove_run_entry();
    Ok("Aguardar Google Drive: DESATIVADO.".to_string())
}

pub fn status() -> Result<String, String> {
    let config = load_config();
    if !config.enabled {
        return Ok("Aguardar Google Drive: DESATIVADO".to_string());
    }
    Ok(format!(
        "Aguardar Google Drive: ATIVADO\nPerfil que será restaurado: {}\nTimeout: {}",
        config.profile.unwrap_or_else(|| "(não definido)".to_string()),
        config
            .timeout_secs
            .map(|t| format!("ativado ({t}s)"))
            .unwrap_or_else(|| "desativado (sem limite)".to_string())
    ))
}
