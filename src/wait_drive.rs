use std::env;
use std::path::PathBuf;

use crate::config::{self, WaitDriveMirror};
use crate::i18n::{t, t_args};

use windows::core::w;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_CREATE_KEY_DISPOSITION, REG_DWORD,
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
            return Err(t_args(
                "wait_drive.error_open_key",
                &[("error", &format!("{status:?}"))],
            ));
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
                return Err(t_args(
                    "wait_drive.error_create_key",
                    &[("error", &format!("{status:?}"))],
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
            return Err(t_args(
                "wait_drive.error_write",
                &[("error", &format!("{status:?}"))],
            ));
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
            return Err(t_args(
                "wait_drive.error_write",
                &[("error", &format!("{status:?}"))],
            ));
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
        if RegQueryValueExW(
            key,
            name,
            None,
            None,
            Some(buf.as_mut_ptr()),
            Some(&mut size),
        )
        .is_err()
        {
            return None;
        }
        let wide = std::slice::from_raw_parts(buf.as_ptr() as *const u16, (size as usize) / 2);
        let text = String::from_utf16_lossy(wide)
            .trim_end_matches('\0')
            .to_string();
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
    // Consulta rápida: lê o espelho no config.json (evita abrir o registro).
    if let Some(mirror) = config::get_wait_drive() {
        return WaitDriveConfig {
            enabled: mirror.enabled,
            profile: mirror.profile,
            timeout_secs: mirror.timeout_secs,
        };
    }
    // Sem espelho: cai no registro e autopopula o config.json.
    let mut config = WaitDriveConfig::default();
    if let Ok(key) = open_app_key(KEY_QUERY_VALUE) {
        config.profile = query_sz(key, VALUE_PROFILE);
        config.timeout_secs =
            query_dword(key, VALUE_TIMEOUT).and_then(
                |v| {
                    if v == 0 {
                        None
                    } else {
                        Some(v as u64)
                    }
                },
            );
        config.enabled = config.profile.is_some();
        unsafe {
            let _ = RegCloseKey(key);
        }

        config::set_wait_drive(Some(WaitDriveMirror {
            enabled: config.enabled,
            profile: config.profile.clone(),
            timeout_secs: config.timeout_secs,
        }));
    }
    config
}

pub fn enable(profile: &str, timeout_secs: Option<u64>) -> Result<String, String> {
    let key = open_app_key(KEY_SET_VALUE)?;
    let result = set_sz(key, VALUE_PROFILE, profile)
        .and_then(|_| set_dword(key, VALUE_TIMEOUT, timeout_secs.unwrap_or(0) as u32));
    unsafe {
        let _ = RegCloseKey(key);
    }
    result?;
    register_run_entry()?;
    // Espelha a configuração no config.json (o registro continua a fonte real).
    config::set_wait_drive(Some(WaitDriveMirror {
        enabled: true,
        profile: Some(profile.to_string()),
        timeout_secs,
    }));
    Ok(t_args(
        "wait_drive.enable_success",
        &[
            ("profile", profile),
            ("timeout", &timeout_text(timeout_secs)),
        ],
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
    config::set_wait_drive(Some(WaitDriveMirror {
        enabled: false,
        profile: None,
        timeout_secs: None,
    }));
    Ok(t("wait_drive.disabled"))
}

pub fn status() -> Result<String, String> {
    let config = load_config();
    if !config.enabled {
        return Ok(t("wait_drive.status_disabled"));
    }
    let profile = config
        .profile
        .unwrap_or_else(|| t("wait_drive.profile_undefined"));
    let timeout = config
        .timeout_secs
        .map(|s| t_args("wait_drive.timeout_on", &[("seconds", &s.to_string())]))
        .unwrap_or_else(|| t("wait_drive.timeout_off"));
    Ok(t_args(
        "wait_drive.status_enabled",
        &[("profile", &profile), ("timeout", &timeout)],
    ))
}

fn timeout_text(timeout_secs: Option<u64>) -> String {
    timeout_secs
        .map(|s| format!("{s}s"))
        .unwrap_or_else(|| t("wait_drive.timeout_off"))
}
