use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use windows::core::Result;
use windows::core::PWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Controls::{
    LVIF_TEXT, LVITEMW, LVM_GETITEMCOUNT, LVM_GETITEMPOSITION, LVM_GETITEMTEXTW,
    LVM_SETITEMPOSITION,
};
use windows::Win32::UI::WindowsAndMessaging::SendMessageW;

use crate::remote_memory::{ProcessHandle, RemoteBuffer};
use crate::shell_locator;
use crate::types::{DesktopProfile, IconEntry};

const TEXT_BUF_CHARS: usize = 512;

pub fn save_layout(profile_name: &str) -> Result<DesktopProfile> {
    let listview = shell_locator::find_desktop_listview()?;
    let process = shell_locator::open_explorer_process(listview)?;

    let count = unsafe { SendMessageW(listview, LVM_GETITEMCOUNT, None, LPARAM(0)).0 } as usize;

    let point_buf = RemoteBuffer::new(&process, std::mem::size_of::<POINT>())?;
    let item_buf = RemoteBuffer::new(&process, std::mem::size_of::<LVITEMW>())?;
    let text_buf = RemoteBuffer::new(&process, TEXT_BUF_CHARS * 2)?;

    let mut icons = Vec::with_capacity(count);
    for index in 0..count {
        let (name, point) = read_item(listview, &process, &point_buf, &item_buf, &text_buf, index)?;
        icons.push(IconEntry {
            name,
            x: point.x,
            y: point.y,
        });
    }

    let timestamp_utc = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();

    Ok(DesktopProfile {
        profile_name: profile_name.to_string(),
        resolution: shell_locator::current_resolution(),
        timestamp_utc,
        icons,
    })
}

pub fn restore_layout(profile: &DesktopProfile) -> Result<usize> {
    let listview = shell_locator::find_desktop_listview()?;
    let process = shell_locator::open_explorer_process(listview)?;

    let count = unsafe { SendMessageW(listview, LVM_GETITEMCOUNT, None, LPARAM(0)).0 } as usize;

    let point_buf = RemoteBuffer::new(&process, std::mem::size_of::<POINT>())?;
    let item_buf = RemoteBuffer::new(&process, std::mem::size_of::<LVITEMW>())?;
    let text_buf = RemoteBuffer::new(&process, TEXT_BUF_CHARS * 2)?;

    let mut name_to_index: HashMap<String, usize> = HashMap::with_capacity(count);
    for index in 0..count {
        let (name, _) = read_item(listview, &process, &point_buf, &item_buf, &text_buf, index)?;
        name_to_index.insert(name, index);
    }

    let mut moved = 0usize;
    for icon in &profile.icons {
        if let Some(&index) = name_to_index.get(&icon.name) {
            let lparam = ((icon.y as u16 as u32) << 16 | (icon.x as u16 as u32)) as isize;
            unsafe {
                SendMessageW(
                    listview,
                    LVM_SETITEMPOSITION,
                    WPARAM(index),
                    LPARAM(lparam),
                );
            }
            moved += 1;
        }
    }

    shell_locator::refresh(listview);
    Ok(moved)
}

fn read_item(
    listview: HWND,
    _process: &ProcessHandle,
    point_buf: &RemoteBuffer,
    item_buf: &RemoteBuffer,
    text_buf: &RemoteBuffer,
    index: usize,
) -> Result<(String, POINT)> {
    unsafe {
        SendMessageW(
            listview,
            LVM_GETITEMPOSITION,
            WPARAM(index),
            LPARAM(point_buf.as_ptr() as isize),
        );
    }
    let point: POINT = point_buf.read()?;

    let item = LVITEMW {
        mask: LVIF_TEXT,
        iItem: index as i32,
        pszText: PWSTR(text_buf.as_ptr() as *mut u16),
        cchTextMax: TEXT_BUF_CHARS as i32,
        ..Default::default()
    };
    item_buf.write(&item)?;

    unsafe {
        SendMessageW(
            listview,
            LVM_GETITEMTEXTW,
            WPARAM(index),
            LPARAM(item_buf.as_ptr() as isize),
        );
    }
    let name = text_buf.read_wide_string()?;

    Ok((name, point))
}
