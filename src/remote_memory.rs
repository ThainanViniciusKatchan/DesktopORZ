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

use std::ffi::c_void;

use windows::core::Result;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};

pub struct ProcessHandle(HANDLE);

impl ProcessHandle {
    pub fn new(handle: HANDLE) -> Self {
        Self(handle)
    }

    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub struct RemoteBuffer<'a> {
    process: &'a ProcessHandle,
    address: *mut c_void,
    size: usize,
}

impl<'a> RemoteBuffer<'a> {
    pub fn new(process: &'a ProcessHandle, size: usize) -> Result<Self> {
        let address = unsafe {
            VirtualAllocEx(
                process.raw(),
                None,
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        if address.is_null() {
            return Err(windows::core::Error::from_win32());
        }
        Ok(Self {
            process,
            address,
            size,
        })
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.address
    }

    pub fn write<T: Copy>(&self, value: &T) -> Result<()> {
        unsafe {
            WriteProcessMemory(
                self.process.raw(),
                self.address,
                value as *const T as *const c_void,
                std::mem::size_of::<T>(),
                None,
            )
        }
    }

    pub fn read<T: Copy + Default>(&self) -> Result<T> {
        let mut value = T::default();
        unsafe {
            ReadProcessMemory(
                self.process.raw(),
                self.address,
                &mut value as *mut T as *mut c_void,
                std::mem::size_of::<T>(),
                None,
            )?;
        }
        Ok(value)
    }

    pub fn read_wide_string(&self) -> Result<String> {
        let mut buf = vec![0u16; self.size / 2];
        unsafe {
            ReadProcessMemory(
                self.process.raw(),
                self.address,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() * 2,
                None,
            )?;
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(String::from_utf16_lossy(&buf[..end]))
    }
}

impl<'a> Drop for RemoteBuffer<'a> {
    fn drop(&mut self) {
        unsafe {
            let _ = VirtualFreeEx(self.process.raw(), self.address, 0, MEM_RELEASE);
        }
    }
}
