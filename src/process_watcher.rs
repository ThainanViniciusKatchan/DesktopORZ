use std::thread::sleep;
use std::time::{Duration, Instant};

use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};

/// Nome do executável do Google Drive para desktop.
pub const GOOGLE_DRIVE_PROCESS: &str = "GoogleDriveFS.exe";

/// Retorna true se um processo com o nome (case-insensitive) estiver rodando.
pub fn is_process_running(process_name: &str) -> bool {
    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(handle) => handle,
            Err(_) => return false,
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut found = false;
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if name.eq_ignore_ascii_case(process_name) {
                    found = true;
                    break;
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
        found
    }
}

/// Espera o processo aparecer e depois aguarda um tempo de estabilização
/// para que ele termine de criar/modificar os ícones da área de trabalho.
/// Retorna quanto tempo esperou até o processo ser detectado.
pub fn wait_for_process(
    process_name: &str,
    timeout: Option<Duration>,
    settle: Duration,
) -> Result<Duration, String> {
    let start = Instant::now();
    let poll = Duration::from_secs(1);

    loop {
        if is_process_running(process_name) {
            let waited = start.elapsed();
            if !settle.is_zero() {
                sleep(settle);
            }
            return Ok(waited);
        }
        if let Some(limit) = timeout {
            if start.elapsed() >= limit {
                return Err(format!(
                    "Tempo esgotado: o processo '{process_name}' não foi iniciado em {}s.",
                    limit.as_secs()
                ));
            }
        }
        sleep(poll);
    }
}
