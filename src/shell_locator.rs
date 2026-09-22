use windows::core::{w, Error, Result, PCWSTR};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowThreadProcessId,
};

use crate::remote_memory::ProcessHandle;

pub fn find_desktop_listview() -> Result<HWND> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), PCWSTR::null())?;
        if let Some(listview) = find_listview_under(progman) {
            return Ok(listview);
        }

        let mut worker = HWND::default();
        loop {
            worker = FindWindowExW(HWND::default(), worker, w!("WorkerW"), PCWSTR::null())?;
            if worker.is_invalid() {
                break;
            }
            if let Some(listview) = find_listview_under(worker) {
                return Ok(listview);
            }
        }

        Err(Error::from_win32())
    }
}

fn find_listview_under(parent: HWND) -> Option<HWND> {
    unsafe {
        let def_view = FindWindowExW(parent, HWND::default(), w!("SHELLDLL_DefView"), PCWSTR::null())
            .ok()?;
        if def_view.is_invalid() {
            return None;
        }
        let listview =
            FindWindowExW(def_view, HWND::default(), w!("SysListView32"), PCWSTR::null()).ok()?;
        if listview.is_invalid() {
            return None;
        }
        Some(listview)
    }
}

pub fn open_explorer_process(listview: HWND) -> Result<ProcessHandle> {
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
    };

    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(listview, Some(&mut pid));
        if pid == 0 {
            return Err(Error::from_win32());
        }
        let handle = OpenProcess(
            PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
            false,
            pid,
        )?;
        Ok(ProcessHandle::new(handle))
    }
}

pub fn current_resolution() -> crate::types::Resolution {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe {
        crate::types::Resolution {
            width: GetSystemMetrics(SM_CXSCREEN) as u32,
            height: GetSystemMetrics(SM_CYSCREEN) as u32,
        }
    }
}

pub fn refresh(listview: HWND) {
    use windows::Win32::Graphics::Gdi::HRGN;
    use windows::Win32::Graphics::Gdi::{
        InvalidateRect, RedrawWindow, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW,
    };
    unsafe {
        let _ = InvalidateRect(listview, None::<*const RECT>, true);
        let _ = RedrawWindow(
            listview,
            None,
            HRGN::default(),
            RDW_INVALIDATE | RDW_ERASE | RDW_UPDATENOW,
        );
    }
}
