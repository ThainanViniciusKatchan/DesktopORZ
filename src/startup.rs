use std::env;
use std::path::PathBuf;

use windows::core::w;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ,
};

const RUN_KEY: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE_NAME: windows::core::PCWSTR = w!("desk0k");

fn exe_path() -> Result<PathBuf, String> {
    let p = env::current_exe()
        .map(|p| p.canonicalize().unwrap_or(p))
        .map_err(|e| format!("{e}"))?;
    // No Windows o caminho pode vir com o prefixo estendido `\\?\`, que o
    // gerenciador da chave Run não entende — removemos aqui.
    Ok(strip_unc_prefix(p))
}

fn strip_unc_prefix(p: PathBuf) -> PathBuf {
    if let Some(s) = p.to_str() {
        if let Some(rest) = s.strip_prefix("\\\\?\\") {
            return PathBuf::from(rest);
        }
    }
    p
}

fn command_line(exe: &PathBuf, profile: &str) -> Vec<u16> {
    let mut s: Vec<u16> = format!("\"{}\" restore \"{profile}\"", exe.display())
        .encode_utf16()
        .collect();
    s.push(0);
    s
}

fn open_run_key(access: REG_SAM_FLAGS) -> Result<HKEY, String> {
    unsafe {
        let mut key = HKEY::default();
        let status = RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, 0, access, &mut key);
        if status.is_err() {
            return Err(format!("Não foi possível abrir a chave Run do registro: {status:?}"));
        }
        Ok(key)
    }
}

pub fn enable(profile: &str) -> Result<String, String> {
    let exe = exe_path()?;
    let cmd = command_line(&exe, profile);
    unsafe {
        let key = open_run_key(KEY_SET_VALUE)?;
        let status = RegSetValueExW(
            key,
            VALUE_NAME,
            0,
            REG_SZ,
            Some(std::slice::from_raw_parts(
                cmd.as_ptr() as *const u8,
                cmd.len() * 2,
            )),
        );
        let _ = RegCloseKey(key);
        if status.is_err() {
            return Err(format!("Falha ao gravar no registro: {status:?}"));
        }
    }
    Ok(format!(
        "Inicialização automática ATIVADA. Perfil '{}' será restaurado ao iniciar o Windows.\nCaminho registrado: {}",
        profile,
        exe.display()
    ))
}

pub fn disable() -> Result<String, String> {
    unsafe {
        let key = open_run_key(KEY_SET_VALUE)?;
        let status = RegDeleteValueW(key, VALUE_NAME);
        let _ = RegCloseKey(key);
        if status.is_err() {
            if status == ERROR_FILE_NOT_FOUND {
                return Ok("Inicialização automática já estava desativada.".to_string());
            }
            return Err(format!("Falha ao remover do registro: {status:?}"));
        }
    }
    Ok("Inicialização automática DESATIVADA.".to_string())
}

pub fn status() -> Result<String, String> {
    unsafe {
        let key = open_run_key(KEY_QUERY_VALUE)?;
        let mut size: u32 = 0;
        let query = RegQueryValueExW(key, VALUE_NAME, None, None, None, Some(&mut size));
        if query.is_err() {
            let _ = RegCloseKey(key);
            return Ok("Inicialização automática: DESATIVADA".to_string());
        }
        let mut buf = vec![0u8; size as usize];
        let query = RegQueryValueExW(key, VALUE_NAME, None, None, Some(buf.as_mut_ptr()), Some(&mut size));
        let _ = RegCloseKey(key);
        if query.is_err() {
            return Ok("Inicialização automática: DESATIVADA".to_string());
        }
        let wide = std::slice::from_raw_parts(buf.as_ptr() as *const u16, (size as usize) / 2);
        let text = String::from_utf16_lossy(wide).trim_end_matches('\0').to_string();
        Ok(format!(
            "Inicialização automática: ATIVADA\nComando registrado: {text}"
        ))
    }
}
